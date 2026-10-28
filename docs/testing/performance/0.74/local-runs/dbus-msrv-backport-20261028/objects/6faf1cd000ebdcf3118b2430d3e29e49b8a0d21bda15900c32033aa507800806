//! Async locks and other primitives that suspend the task instead of blocking the thread.
//!
//! * [`Mutex`] and [`RwLock`] are locks whose guards can be held across an await.
//! * [`Semaphore`] is a lock that up to a set number of tasks can hold at once. It counts permits
//!   instead of holding a value. Use it to cap how many tasks do something at once, such as hold a
//!   connection.
//! * [`Barrier`] makes a number of tasks wait for each other.
//! * [`OnceCell`] holds a value that is set once, by an initialiser that may await. Tasks can wait
//!   for the value.
//!
//! The locks of [`std::sync`] do not suit tasks that keep a lock while they wait for something
//! else. Their guards are `!Send`, so a future that holds one cannot move between threads. A
//! contended `std` lock also blocks the thread, along with every other future it drives. The types
//! here suspend only the task. A lock that is held for a few instructions with no await in between
//! can stay a `std` lock.
//!
//! The types are built on [`Event`](crate::Event) and need no runtime. They work under any executor
//! and from any thread, inside a task or outside one. A plain thread can wait for a lock with any
//! executor's `block_on`.
//!
//! A future that waits for a lock, or holds a guard across an await, can move between threads if
//! the lock can be shared between them. That needs a `T` that is `Send` for a [`Mutex`], and `Send`
//! and `Sync` for an [`RwLock`]. A [`Semaphore`] and a [`Barrier`] hold no value, so they can
//! always be shared. A [`OnceCell`] needs a `T` that is `Send` and `Sync`, and an initialiser and
//! future that can move too.
//!
//! Each lock has two kinds of guard. [`Mutex::lock`], [`RwLock::read`] and [`RwLock::write`] give a
//! guard that borrows the lock. [`Mutex::lock_arc`], [`RwLock::read_arc`] and [`RwLock::write_arc`]
//! are called on an `Arc` of the lock. They give a guard that holds a clone of the `Arc` and
//! borrows nothing, so it can be stored in a struct or moved into a spawned task. Both kinds wait
//! and treat other waiting tasks in the same way. A [`Semaphore`] has the same two kinds, through
//! [`Semaphore::acquire`] and [`Semaphore::acquire_arc`].
//!
//! # Example
//!
//! A task on another thread increments a counter and holds the guard across an await. With a lock
//! from [`std::sync`], the future could not be sent to that thread:
//!
//! ```
//! use std::{future, sync::Arc, thread};
//!
//! use futures::executor::block_on;
//! use zruntime::lock::Mutex;
//!
//! let counter = Arc::new(Mutex::new(0));
//! let task = {
//!     let counter = counter.clone();
//!     async move {
//!         let mut guard = counter.lock().await;
//!         let step = future::ready(1).await;
//!         *guard += step;
//!     }
//! };
//!
//! thread::spawn(move || block_on(task))
//!     .join()
//!     .expect("the other thread did not panic");
//!
//! assert_eq!(*block_on(counter.lock()), 1);
//! ```
//!
//! # No poisoning
//!
//! A panic while a guard is held does not poison the lock. The guard is dropped when the panic
//! unwinds, or when the future that holds it is dropped, and this releases the lock. The next task
//! to take it sees the value as the panicking code left it, possibly half-way through an update.
//!
//! * A [`Semaphore`] has no value, so a guard dropped during a panic just gives its permit back.
//! * A [`Barrier`] has no guard. A task that panics instead of arriving leaves the others waiting,
//!   as with [`std::sync::Barrier`].
//! * A [`OnceCell`] is not poisoned either. If an initialiser panics, the cell stays empty and the
//!   next task that asks for the value runs its own initialiser.
//!
//! # Fairness
//!
//! The locks are not strictly fair: tasks do not always get a lock in the order they asked for it.
//!
//! [`Mutex::lock`], [`RwLock::read`], [`RwLock::write`] and [`Semaphore::acquire`] first try to
//! take the lock. If the try succeeds, the task gets the lock ahead of the tasks that are already
//! waiting. A `lock`, `write` or `acquire` succeeds when the lock or a permit is free, and a `read`
//! succeeds when no writer holds the lock or waits for it. This keeps a contended lock busy: the
//! running task can take the lock as it is released, instead of the lock staying free until a
//! waiting task has been woken and has run. The `_arc` calls, such as [`Mutex::lock_arc`], work in
//! the same way.
//!
//! A waiting task that is woken and finds the lock taken again goes back to waiting, behind the
//! tasks that started waiting after it did.
//!
//! Newcomers are calls that have not waited yet. They can keep a task waiting only for a limited
//! time. A task that has waited for a while, and is then woken only to find the lock taken again,
//! holds newcomers back until it gets the lock or gives up:
//!
//! * A [`Mutex`] or [`Semaphore`] that such a task waits for is not given to a newcomer, even while
//!   the mutex is free or the semaphore has free permits. A newcomer `lock` or `acquire` waits
//!   behind the task. [`try_lock`](Mutex::try_lock) and [`try_acquire`](Semaphore::try_acquire)
//!   return `None`. A task that was already waiting can still take the lock ahead of the task that
//!   holds newcomers back, which then waits again, behind every task that has started waiting
//!   since. For a semaphore, a newcomer that checks at the very moment the task starts to hold
//!   newcomers back can still slip past it.
//! * An [`RwLock`] that such a writer waits for is not given to a newcomer `write`, in the same
//!   way, or to [`try_write`](RwLock::try_write).
//! * A reader that has waited for a while, and is then woken only to find a writer holding the
//!   [`RwLock`] or waiting for it, is let in the next time no writer holds the lock. It goes ahead
//!   of the waiting writers, and no writer takes the lock until the reader is in. Other readers are
//!   still held back by a waiting writer: see [write preference](#write-preference).
//!
//! On targets where the standard library has no clock, such as `wasm32-unknown-unknown`, a task
//! cannot tell how long it has waited. There, a task holds newcomers back the first time it is
//! woken and finds the lock taken.
//!
//! The tasks that initialise a [`OnceCell`] take turns in the order that a [`Mutex`] serves its
//! waiters.
//!
//! # Write preference
//!
//! An [`RwLock`] prefers writers: while a writer waits for the lock, new readers wait too. A task
//! that holds a read guard must therefore not ask for another one, because that can wait forever.
//! See [write preference](RwLock#write-preference).
//!
//! # Giving up a wait
//!
//! Dropping a future before it completes gives up the wait, which is safe: a timeout or a `select`
//! can do it. The lock is not taken, and no other waiting task is left stranded. This holds for
//! [`Mutex::lock`], [`RwLock::read`], [`RwLock::write`], their `_arc` variants,
//! [`Semaphore::acquire`] and [`Semaphore::acquire_arc`]. A dropped `acquire` future takes no
//! permit.
//!
//! Dropping the future of [`Barrier::wait`] after its first poll and before it completes withdraws
//! the task's arrival, unless the round has already completed. The barrier then needs that arrival
//! again, so nobody is released early. If the round completed before the drop, the dropped task is
//! not counted towards the next round.
//!
//! Dropping the future of [`OnceCell::get_or_init`], [`OnceCell::get_or_try_init`] or
//! [`OnceCell::set`] gives up the call. If the call was running the initialiser, the initialiser's
//! future is dropped with it, the cell stays empty, and the next task waiting to initialise the
//! cell runs its own initialiser. Dropping the future of [`OnceCell::wait`] gives up the wait and
//! affects no other task.

mod barrier;
mod mutex;
mod once_cell;
mod rwlock;
mod semaphore;

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
use std::time::{Duration, Instant};

pub use barrier::{Barrier, BarrierWaitResult};
pub use mutex::{Mutex, MutexGuard, MutexGuardArc};
pub use once_cell::OnceCell;
pub use rwlock::{
    RwLock, RwLockReadGuard, RwLockReadGuardArc, RwLockWriteGuard, RwLockWriteGuardArc,
};
pub use semaphore::{Semaphore, SemaphoreGuard, SemaphoreGuardArc};

/// When a `lock`, `read`, `write` or `acquire` call began to wait, so that it can tell once it has
/// waited for long enough to hold newcomers back.
///
/// Where the standard library has no clock, as on `wasm32-unknown-unknown`, it keeps no time, and
/// a call has waited for long enough as soon as it has waited at all: it then holds newcomers back
/// the first time it is woken only to find the lock taken. There is no thread there to hand the
/// lock over to, so nothing is saved by letting the running task take it again first.
#[derive(Clone, Copy)]
struct WaitStart {
    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    at: Instant,
}

impl WaitStart {
    fn now() -> Self {
        Self {
            #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
            at: Instant::now(),
        }
    }

    /// Whether the call has waited for long enough to hold newcomers back, the next time it is
    /// woken and cannot get in.
    fn waited_long(self) -> bool {
        #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
        return self.at.elapsed() >= PATIENCE;
        #[cfg(all(target_family = "wasm", target_os = "unknown"))]
        return true;
    }
}

/// How long a task waits for a lock before it may hold newcomers back.
///
/// Long next to a handoff of the lock between two threads, and short next to a delay a task would
/// notice: for this long, a lock under steady contention keeps going to whichever task is running
/// when it is released, rather than to a waiter that first has to be woken and scheduled. The same
/// as async-lock's.
#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
pub(crate) const PATIENCE: Duration = Duration::from_micros(500);
