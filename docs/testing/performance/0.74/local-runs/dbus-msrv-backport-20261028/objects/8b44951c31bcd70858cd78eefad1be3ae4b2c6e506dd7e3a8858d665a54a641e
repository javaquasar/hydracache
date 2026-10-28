//! Blocking work on a pool of threads, and [`Unblock`], an adapter for blocking I/O handles.
//!
//! [`unblock()`] runs a closure on a pool of threads and returns a future of its result.
//! [`Unblock`] gives a blocking I/O handle (a file, the standard input, an iterator) async read,
//! write, seek and stream traits by running each operation on it as blocking work.
//!
//! Neither needs a runtime. Both work under any executor.
//!
//! Neither is built on WASI, where blocking work has no thread to run on: see the `WASI` section
//! of each item.

/// The `WASI` section of the documentation of each item that runs blocking work on the pool, and
/// that is therefore not built on WASI.
macro_rules! not_on_wasi {
    () => {
        "\n\n# WASI\n\n\
        Not available on WASI. Blocking work needs a thread of its own to run on, out of the way \
        of the thread that polls tasks, and `wasm32-wasip1` and `wasm32-wasip2` cannot start a \
        thread. Running the work on the thread that polls its future instead would block that \
        thread, and every task on it, until the work is done, which is no async at all.\n\n\
        This can change once a WASI target with threads can be told apart from one without: \
        `wasm32-wasip1-threads` can start threads, but no `cfg` sets it apart from \
        `wasm32-wasip1`."
    };
}

mod io;
pub(crate) mod pool;

use std::{
    any::Any,
    fmt,
    future::Future,
    mem,
    num::NonZeroUsize,
    panic::{self, AssertUnwindSafe},
    pin::Pin,
    sync::{Arc, Mutex, MutexGuard, PoisonError, TryLockError},
    task::{Context, Poll, Waker},
    time::Duration,
};

pub use io::Unblock;
use pool::Pool;

/// Runs `work` on a thread of a pool kept for blocking work, and returns a future of its result.
///
/// Use it for work that blocks, such as a file read, a host-name lookup or a call into a library
/// with no async form. If such work runs on the thread that polls tasks, it holds up every other
/// task on that thread. Here it holds up only the pool thread that runs it. The task that awaits
/// the future is woken when the work is done.
///
/// The work is submitted to the pool before `unblock` returns, not when the future is first polled.
/// An idle pool thread takes it. If none is idle, a new thread is started, up to a limit of 500
/// threads. Beyond that, the work waits in a queue, in submission order, until a thread is free.
///
/// Pool threads are named `zruntime blocking work`. A thread is reused for later work. It exits
/// after ten seconds with nothing to run, so an unused pool holds no threads.
///
/// Work that never ends holds its thread forever. If every pool thread is held that way, later work
/// waits forever too. The pool suits work that ends on its own. For work that waits on other pool
/// work, or that runs as long as the program does, use [`std::thread::spawn`] instead.
///
/// The work runs to the end whether or not the future is polled, and even if the future is dropped.
/// Dropping the future gives up the wait for the result. It cannot cancel the work, or remove it
/// from the queue before it starts. Nothing waits for the pool threads, so work that is still
/// running or queued when the process exits is lost.
///
/// The returned future needs no runtime. It wakes the waker from its most recent poll, so it can be
/// awaited under any executor, from a task on any thread.
///
/// # Panics
///
/// If `work` panics, the panic is caught on the pool thread, which carries on with other work. The
/// poll that would have returned the value raises the panic again, with its original payload. A
/// future that is dropped before that poll never sees the panic. The panic hook runs once, when the
/// panic happens, and not again when the panic is raised again.
///
/// `unblock` itself panics if the pool has no thread and cannot start one for the work, as
/// [`std::thread::spawn`] does when it fails. If the pool has threads, the work waits for one of
/// them instead.
///
/// # Example
///
/// A sleep stands in for blocking work. The example drives the future with `block_on` from the
/// `futures` crate, but the `block_on` of any executor works, as `unblock` needs no runtime:
///
/// ```
/// use std::{thread, time::Duration};
///
/// use futures::executor::block_on;
/// use zruntime::unblock;
///
/// let answer = unblock(|| {
///     thread::sleep(Duration::from_millis(10));
///     42
/// });
///
/// assert_eq!(block_on(answer), 42);
/// ```
#[doc = not_on_wasi!()]
pub fn unblock<T>(work: impl FnOnce() -> T + Send + 'static) -> BlockingWork<T>
where
    T: Send + 'static,
{
    unblock_on(&POOL, work)
}

/// The future returned by [`unblock()`]. It resolves to the value the work returned.
///
/// It needs no runtime and works under any executor. It is `Send` and `Sync` if the value is
/// `Send`, so a task that awaits it can move between threads.
///
/// Dropping it before it resolves gives up the wait, not the work. The pool still runs the work to
/// the end, in its turn if no thread has started it yet. Once the future is dropped or has
/// resolved, the work no longer keeps the awaiting task or its executor alive.
#[doc = not_on_wasi!()]
pub struct BlockingWork<T>(Arc<Mutex<State<T>>>);

impl<T> Future for BlockingWork<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        let mut guard = lock(&self.0);
        let Some(outcome) = guard.outcome.take() else {
            let waker = cx.waker();
            let replaced = match guard.waker.as_ref() {
                Some(stored) if stored.will_wake(waker) => None,
                _ => guard.waker.replace(waker.clone()),
            };
            drop(guard);
            // Past the lock, which belongs to this hand-over alone and is none of a dropped
            // waker's business.
            drop(replaced);
            return Poll::Pending;
        };
        // Taking the waker here too, whether or not this poll needed it, keeps a future that
        // resolves without ever being woken from leaving one behind for `Finish` to wake later,
        // once whoever it belongs to may already be gone.
        let leftover = guard.waker.take();
        drop(guard);
        drop(leftover);

        match outcome {
            Ok(value) => Poll::Ready(value),
            Err(panic) => panic::resume_unwind(panic),
        }
    }
}

impl<T> Drop for BlockingWork<T> {
    fn drop(&mut self) {
        // Besides this future, only the thread that runs the work takes the lock, and only once
        // the work is over: to store the outcome, and then in `Finish`, which takes the waker out
        // and wakes it. So a lock this drop finds taken leaves the waker with that thread only
        // until `Finish` has woken it, moments later, and waiting for the lock instead could wait
        // for good: a `wake` from inside `Finish` may drop this very future on that thread, as an
        // executor does with a task it can no longer run.
        let mut state = match self.0.try_lock() {
            Ok(state) => state,
            Err(TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
            Err(TryLockError::WouldBlock) => return,
        };
        let waker = state.waker.take();
        drop(state);
        // Past the lock, as in `poll`.
        drop(waker);
    }
}

impl<T> fmt::Debug for BlockingWork<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BlockingWork").finish_non_exhaustive()
    }
}

/// Runs `work` as [`unblock()`] does, but on a thread of `pool` rather than of the pool that
/// `unblock()` hands its work to, and hands back a future of what it returns.
///
/// All that `unblock()` says of the work and of its future holds here with `pool` in place of that
/// pool: the work is handed to `pool` before this returns, and waits its turn there once `pool` has
/// as many threads as its cap allows.
///
/// # Panics
///
/// Panics if `pool` has no thread at all and cannot start one for the work, as [`unblock()`] does.
pub(crate) fn unblock_on<T>(
    pool: &'static Pool,
    work: impl FnOnce() -> T + Send + 'static,
) -> BlockingWork<T>
where
    T: Send + 'static,
{
    let state = Arc::new(Mutex::new(State {
        outcome: None,
        waker: None,
    }));

    let job_state = state.clone();
    Pool::submit(
        pool,
        Box::new(move || {
            // Constructed before `work` runs and dropped only once this closure returns, so it
            // covers every way out of the work, storing the outcome of it included.
            let _finish = Finish(&job_state);

            let outcome = panic::catch_unwind(AssertUnwindSafe(work));
            lock(&job_state).outcome = Some(outcome);
        }),
    );

    BlockingWork(state)
}

/// The pool that [`unblock()`] hands work to. Its threads are named `zruntime blocking work`, as
/// the documentation of `unblock()` says.
static POOL: Pool = Pool::new("zruntime blocking work", MAX_THREADS, IDLE_TIMEOUT);

/// The most threads a pool has at once, [`POOL`] and the pool that the waits for child processes
/// run on alike: as many as the pool of smol's `blocking` crate has by default. That is room for a
/// burst of slow lookups or file reads to run side by side, or for a burst of children to be waited
/// for at once, while a flood of work that blocks for good queues up rather than starting threads
/// without end.
pub(crate) const MAX_THREADS: NonZeroUsize = NonZeroUsize::new(500).unwrap();

/// How long a thread of a pool waits for more work before it ends, whichever pool it is: as long
/// as tokio keeps an idle thread of its own pool for blocking work. That spans the gaps in a
/// steady stream of work, and lets the threads that a burst of it started go soon after the burst
/// is over.
#[cfg(not(miri))]
pub(crate) const IDLE_TIMEOUT: Duration = Duration::from_secs(10);

/// Under Miri, a thread of a pool ends as soon as it finds no work to run. Miri ends a program
/// with an error if a thread other than the main one is still running as the main one returns, and
/// a thread that waited for more work would be.
#[cfg(miri)]
pub(crate) const IDLE_TIMEOUT: Duration = Duration::ZERO;

/// Where the thread leaves the outcome of the work, and the waker it hands back to.
///
/// Both live behind the one lock, so that seeing the outcome and taking the waker to wake it are
/// never two separate steps from the other side's point of view: see [`Finish`] for what that
/// buys.
struct State<T> {
    /// The outcome of the work, once the thread has produced it.
    outcome: Option<Outcome<T>>,
    /// The waker of the task that polled for that outcome last.
    waker: Option<Waker>,
}

/// What the work returned, or what it panicked with.
type Outcome<T> = Result<T, Box<dyn Any + Send>>;

/// Wakes whoever is waiting for the outcome as the work ends, before letting go of the lock the
/// waiting side has to take to see that outcome.
///
/// While this drop holds [`State`]'s lock, [`BlockingWork`] cannot take it and so cannot see the
/// outcome stored under it; by the time it can, the waker this drop took out has been woken and,
/// with nothing else left holding it, dropped. A poll that reaches the lock first, in the gap
/// between the outcome being stored and this drop running, takes the waker itself instead,
/// leaving this drop nothing to wake. Either way, the awaiting task can only complete once the
/// thread of the work holds nothing that came from the runtime that polled it, and that runtime is
/// then free to be torn down with nothing of it left on that thread.
///
/// Waking while still holding the lock cannot deadlock here: the lock is private to this
/// hand-over, reachable from nowhere but the job that [`unblock()`] hands the pool and the future
/// it wakes. A `wake` that drops that future, as an executor may with a task it can no longer run,
/// finds the lock taken and leaves it be, the waker having been taken out already: see
/// [`BlockingWork`]'s drop.
struct Finish<'a, T>(&'a Mutex<State<T>>);

impl<T> Drop for Finish<'_, T> {
    fn drop(&mut self) {
        let mut state = lock(self.0);
        if let Some(waker) = state.waker.take() {
            waker.wake();
        }
    }
}

/// The state behind its lock, taken whether or not a panic poisoned it.
fn lock<T>(state: &Mutex<State<T>>) -> MutexGuard<'_, State<T>> {
    state.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Drops the payload of a panic that is to go no further, with a panic of its destructor caught.
///
/// The payload is somebody else's value, and its `Drop` may panic in turn. The payload of such a
/// second panic is dropped too, the same way, and only one that panics a third time is leaked, so
/// as not to follow a chain of destructors that each panic. `Event` has a function like this one
/// too, but this module does not use it: the `event` feature may be left out of the build, and
/// blocking work needs none of it.
fn dispose(payload: Box<dyn Any + Send>) {
    let Err(payload) = panic::catch_unwind(AssertUnwindSafe(move || drop(payload))) else {
        return;
    };
    if let Err(payload) = panic::catch_unwind(AssertUnwindSafe(move || drop(payload))) {
        mem::forget(payload);
    }
}
