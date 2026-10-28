//! A runtime for a connection that runs on one thread and starts none of its own.
//!
//! Readiness comes from a `polling::Poller`, tasks from `async-task`, and timers from a map of
//! deadlines the runtime walks itself. [`Runtime::run`] drives all of it, together with the
//! caller's own future, on the thread that calls it: nothing here starts a thread, so a thread the
//! process gains while a connection is alive is a thread the connection asked for.
//!
//! [`traits::Runtime::connect_tcp`] and [`traits::Runtime::connect_unix`] are implemented here, on
//! the runtime's own poller and timer: a connect that the kernel completes in the background is
//! waited for as any other source is, and a unix connect turned away by a full backlog is tried
//! again after a wait on the timer. A runtime with no non-blocking connect of its own connects on
//! a thread for blocking work instead, and so starts one for the connection to a bus.
//!
//! [`traits::Runtime::spawn_process`] is implemented here too, on the runtime's own timer: a
//! runtime with a pidfd, a kqueue or a handler for `SIGCHLD` is told when a child exits, and this
//! one has nothing of the kind, so it looks at the child at an interval. A connection awaits the
//! exit of a `unixexec:` child on a task of the runtime, which keeps looking until the child has
//! exited, and a released runtime drops that task and leaves the child to the process's own exit.
//! A runtime that waits for a child on a thread for blocking work instead starts one wherever a
//! connection runs a program: on macOS the session bus is found through `launchd`, which does.
//!
//! [`traits::Runtime::spawn_blocking`] keeps its default, which hands each call to zruntime's
//! pool of threads for blocking work. On Linux and Android a connection looks its peer's
//! supplementary groups up through it — [`zbus::Connection::peer_creds`] does, and so does the
//! handshake of a peer-to-peer server, which `teardown` runs — and the default may start a thread
//! for each such call. A runtime with a pool of its own should override the hook rather than copy
//! this.

use std::{
    collections::VecDeque,
    future::Future,
    net::SocketAddr,
    path::Path,
    pin::{Pin, pin},
    process::{Command, ExitStatus, Stdio},
    sync::{Arc, Mutex, MutexGuard, PoisonError, Weak},
    task::{Context, Poll, Wake, Waker},
    time::{Duration, Instant},
};

use async_task::Runnable;
use polling::{Events, Poller};
use zbus::runtime::{IoSource, traits};

mod connect;
mod io;
pub use io::RegisteredIoSource;
use io::Sources;
mod process;
mod task;
use task::Detached;
pub use task::Task;
mod timer;
pub use timer::Sleep;
use timer::Timers;

/// A runtime: a poller, a queue of runnables and a map of timers, driven by one thread.
///
/// Releasing the runtime cancels every task it still holds, whether the task is waiting in the
/// queue as a runnable, parked in the timer or source maps as a waker, or detached and left in
/// the runtime's keeping.
pub struct Runtime(Arc<Inner>);

impl Runtime {
    /// A runtime with a poller of its own.
    pub fn new() -> std::io::Result<Self> {
        let inner = Inner {
            poller: Poller::new()?,
            queue: Mutex::new(VecDeque::new()),
            detached: Mutex::default(),
            timers: Mutex::default(),
            sources: Mutex::default(),
        };

        Ok(Self(Arc::new(inner)))
    }

    /// The runtime to hand to [`zbus::connection::Builder::runtime`].
    pub fn handle(&self) -> Handle {
        Handle(self.0.clone())
    }

    /// Runs `future` on this thread, along with every task, timer and socket of this runtime.
    ///
    /// Each turn runs what the queue holds, polls `future`, and then sleeps in the poller until
    /// a source is ready, a timer is due or a wakeup arrives.
    pub fn run<F>(&self, future: F) -> F::Output
    where
        F: Future,
    {
        let inner = &self.0;
        let waker = Waker::from(Arc::new(Notify(inner.clone())));
        let mut cx = Context::from_waker(&waker);
        let mut future = pin!(future);
        let mut events = Events::new();

        loop {
            loop {
                // The runnable is taken before it runs: running it may queue another one.
                let next = lock(&inner.queue).pop_front();
                match next {
                    Some(runnable) => {
                        runnable.run();
                    }
                    None => break,
                }
            }

            // A detached task is the runtime's to hold until it ends, and only the runtime can
            // notice that it has: letting the finished ones go here keeps that hold to what is
            // running.
            //
            // The finished ones leave the list under the lock and are released after it, because
            // a task that has ended carries what it produced and that value's destructor is free
            // to reach back in: a `Handle` handed on to it can spawn and detach a task of its
            // own, and `detach` wants this very lock.
            let finished = {
                let mut detached = lock(&inner.detached);
                let (finished, running): (Vec<_>, Vec<_>) = std::mem::take(&mut *detached)
                    .into_iter()
                    .partition(|task| task.is_finished());
                *detached = running;

                finished
            };
            drop(finished);

            if let Poll::Ready(output) = future.as_mut().poll(&mut cx) {
                return output;
            }

            let timeout = self.timeout();
            events.clear();
            inner
                .poller
                .wait(&mut events, timeout)
                .expect("the poller waits for readiness");

            for event in events.iter() {
                self.wake_source(event.key);
            }
            self.wake_due_timers();
        }
    }

    /// How many timers are still waiting for their deadline.
    pub fn pending_timers(&self) -> usize {
        lock(&self.0.timers).pending.len()
    }

    /// A watch on the state this runtime shares with the handles it hands out.
    ///
    /// The watch itself keeps nothing alive, so it can be taken before the runtime is released
    /// and asked afterwards whether anything outlived it.
    pub fn probe(&self) -> Probe {
        Probe(Arc::downgrade(&self.0))
    }

    /// How long the poller may sleep: until the earliest deadline, and not at all while there
    /// are runnables waiting to run.
    fn timeout(&self) -> Option<Duration> {
        if !lock(&self.0.queue).is_empty() {
            return Some(Duration::ZERO);
        }

        let timers = lock(&self.0.timers);
        let (deadline, _) = timers.pending.keys().next()?;

        Some(deadline.saturating_duration_since(Instant::now()))
    }
}

/// Releases the tasks the runtime still holds, which nothing else can.
///
/// A task's future owns the [`RegisteredIoSource`]s and [`Sleep`]s it is waiting on, and each of
/// those holds the shared state; the shared state holds the task right back, as a runnable in the
/// queue, as a waker in the timer or source maps, or as a detached task the runtime keeps. That
/// ring keeps itself alive, and the runtime is the only party that knows there is no longer anyone
/// to run it, so it cuts the ring here.
///
/// One pass is not enough: dropping a runnable drops its future, which drops the handles on any
/// task that future spawned, and cancelling a task hands its runnable back to the queue so that
/// its own future is dropped by whoever owns the queue. Each pass therefore takes what is there,
/// drops it, and looks again, until a pass finds nothing left.
///
/// Nothing taken is dropped while a lock is held, because those drops reach back in: a runnable's
/// drop can call the schedule closure, and a future's drop can delete a source.
impl Drop for Runtime {
    fn drop(&mut self) {
        loop {
            let queued = std::mem::take(&mut *lock(&self.0.queue));
            let detached = std::mem::take(&mut *lock(&self.0.detached));
            let timers = std::mem::take(&mut lock(&self.0.timers).pending);
            let states: Vec<_> = lock(&self.0.sources).states.values().cloned().collect();
            let wakers: Vec<_> = states
                .iter()
                .flat_map(|state| {
                    let mut wakers = lock(&state.wakers);

                    [wakers.readable.take(), wakers.writable.take()]
                })
                .flatten()
                .collect();

            if queued.is_empty() && detached.is_empty() && timers.is_empty() && wakers.is_empty() {
                break;
            }

            drop(wakers);
            drop(timers);
            drop(detached);
            drop(queued);
        }
    }
}

/// A watch on a [`Runtime`]'s shared state, handed out by [`Runtime::probe`].
pub struct Probe(Weak<Inner>);

impl Probe {
    /// Whether the state the runtime shared with its handles is gone.
    ///
    /// False while anything at all still holds it: a handle, a registered source, a timer, or a
    /// task that was never dropped.
    pub fn is_released(&self) -> bool {
        self.0.strong_count() == 0
    }
}

/// What a [`Runtime`] hands to a connection: a shared handle on its poller, queue and timers.
#[derive(Clone)]
pub struct Handle(Arc<Inner>);

impl traits::Runtime for Handle {
    type RegisteredIoSource = RegisteredIoSource;
    type Sleep = Sleep;
    type Task<T>
        = Task<T>
    where
        T: Send + 'static;

    fn register_io_source(&self, source: IoSource) -> std::io::Result<RegisteredIoSource> {
        RegisteredIoSource::new(&self.0, source)
    }

    fn sleep(&self, duration: Duration) -> Sleep {
        Sleep::new(self.0.clone(), duration)
    }

    fn spawn<T>(&self, _name: &str, future: impl Future<Output = T> + Send + 'static) -> Task<T>
    where
        T: Send + 'static,
    {
        // This runtime has nowhere to record a task's name, so the name goes no further than
        // here. A queued runnable owns this closure, so the hold on the runtime has to be weak:
        // an `Arc` here would keep a released runtime's queue, timers and sources alive for good.
        let inner = Arc::downgrade(&self.0);
        let schedule = move |runnable: Runnable| {
            let Some(inner) = inner.upgrade() else {
                // The runtime is gone, and dropping the runnable cancels the task.
                return;
            };
            lock(&inner.queue).push_back(runnable);
            // The runtime may be asleep in the poller: bring it back for the new runnable.
            let _ = inner.poller.notify();
        };

        let (runnable, task) = async_task::spawn(future, schedule);
        runnable.schedule();

        Task::new(task, Arc::downgrade(&self.0))
    }

    fn connect_tcp(
        &self,
        address: SocketAddr,
    ) -> impl Future<Output = std::io::Result<IoSource>> + Send {
        connect::tcp(self, address)
    }

    fn connect_unix(&self, path: &Path) -> impl Future<Output = std::io::Result<IoSource>> + Send {
        connect::unix(self, path)
    }

    fn spawn_process(
        &self,
        command: Command,
        stdin: Stdio,
        stdout: Stdio,
        stderr: Stdio,
    ) -> std::io::Result<Pin<Box<dyn Future<Output = std::io::Result<ExitStatus>> + Send>>> {
        process::spawn(self, command, stdin, stdout, stderr)
    }
}

/// Everything a [`Runtime`] shares with the handles it hands out.
struct Inner {
    poller: Poller,
    queue: Mutex<VecDeque<Runnable>>,
    detached: Mutex<Vec<Box<dyn Detached>>>,
    timers: Mutex<Timers>,
    sources: Mutex<Sources>,
}

/// The waker the runtime polls the caller's future with.
struct Notify(Arc<Inner>);

impl Wake for Notify {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        let _ = self.0.poller.notify();
    }
}

/// The value behind a lock, taken whether or not a panic poisoned it.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
