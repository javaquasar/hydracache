//! Tests of the locks of [`crate::lock`]: [`Mutex`] and [`RwLock`]. Those of
//! [`Semaphore`](crate::lock::Semaphore) are in the `semaphore` module, and those of the module's
//! [`Barrier`](crate::lock::Barrier) and [`OnceCell`](crate::lock::OnceCell) in the `barrier` and
//! `once_cell` modules, which share the helpers at the end of this one.
//!
//! Most of these drive the futures of a lock by hand, polling each with a waker that goes nowhere,
//! so that every test says exactly who a release lets in and who still waits. The mutex comes
//! first, with who it admits and who it keeps out, how a taker that has waited for long holds
//! newcomers back and whom it does not hold back, what giving up a wait leaves behind, and what
//! `try_lock` does, then the guards that hold an `Arc` of the mutex: that one goes where a borrow
//! cannot, and that the calls handing them out admit, keep out and hold back as the borrowing ones
//! do. The readers-writer lock follows, with who it admits and who it keeps out, what a waiting
//! writer does to the readers behind it, what giving up a wait leaves behind, what the calls that
//! never wait do and do not look at, how a writer that has waited for long holds newcomer writers
//! back, and how a reader that has waited for long is let in ahead of the writers, starved or not,
//! and holds every one of them back until it is in, then its `Arc` guards, as the mutex's. Then
//! come the tests that cover both locks: that a panic while a guard is held releases it, the
//! smaller conveniences (borrowing the value, taking it back, making a lock, printing it), that an
//! `Arc` guard holds its `Arc`, reaches an unsized value and prints as its value, and that a lock
//! may be a trait object.
//!
//! A task has waited for long once it has waited for `PATIENCE`, so the tests that need one sleep
//! for that long after its first poll, which is all the time they take.
//!
//! The last ten drive one lock from many threads at once. The first two check that it never lets a
//! holder in beside one it should keep out and that nothing deadlocks on the way, the next two that
//! a thread that waits for a lock is let in however steadily two others keep taking it, the next
//! three that a release racing the start of a wait is not missed, the next two that a starved taker
//! of a mutex, or a starved writer, giving up as a newcomer starts to wait does not strand it, and
//! the last that a mutex whose waiters' wakers come back into it from their wake still admits one
//! holder at a time and deadlocks nothing. All but the two that end once the waiter is let in are
//! shrunk under Miri, which runs them far more slowly.

use std::{
    fmt,
    future::Future,
    hint,
    panic::{AssertUnwindSafe, catch_unwind},
    pin::{Pin, pin},
    sync::{
        Arc, Barrier,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::{Context, Poll, Wake, Waker},
    thread,
};

use futures::executor::block_on;
use ntest::timeout;

use crate::lock::{Mutex, MutexGuard, PATIENCE, RwLock, RwLockWriteGuard};

mod barrier;
mod once_cell;
mod semaphore;

/// A second taker waits while the first holds the mutex, and is let in once the first lets go.
#[test]
fn a_mutex_admits_one_holder_at_a_time() {
    let mutex = Mutex::new(0);
    let guard = ready(mutex.lock());
    let mut second = Box::pin(mutex.lock());
    assert!(poll_once(&mut second).is_pending());
    drop(guard);
    assert!(poll_once(&mut second).is_ready());
}

/// A newcomer takes a mutex that is free ahead of a taker that has been waiting for it, but not for
/// long: the release woke that taker, which had not run yet when the newcomer came. The waiter
/// finds the mutex taken again and goes back to waiting, and is let in only by the newcomer's
/// release. A waiter that has waited for long is let in before newcomers once it has lost such a
/// race: see the tests that follow.
#[test]
fn a_newcomer_takes_a_free_mutex_ahead_of_a_waiter() {
    let mutex = Mutex::new(());
    let guard = ready(mutex.lock());
    let mut waiter = Box::pin(mutex.lock());
    assert!(poll_once(&mut waiter).is_pending());
    drop(guard);

    let mut newcomer = Box::pin(mutex.lock());
    let Poll::Ready(newcomer) = poll_once(&mut newcomer) else {
        panic!("a free mutex is taken at once, whoever waits for it");
    };

    assert!(poll_once(&mut waiter).is_pending());
    drop(newcomer);
    assert!(poll_once(&mut waiter).is_ready());
}

/// A `lock` future dropped after the release woke it, before it took the mutex, passes the wake-up
/// on, so the taker behind it is let in and not left waiting for a release that never comes.
#[test]
fn a_dropped_lock_future_does_not_strand_the_next_waiter() {
    let mutex = Mutex::new(());
    let guard = ready(mutex.lock());
    let mut first = Box::pin(mutex.lock());
    assert!(poll_once(&mut first).is_pending());
    let mut second = Box::pin(mutex.lock());
    assert!(poll_once(&mut second).is_pending());
    // The release notifies `first`, which is then dropped without ever polling that
    // notification: it has to reach `second` instead.
    drop(guard);
    drop(first);
    assert!(poll_once(&mut second).is_ready());
}

/// A taker that has waited for long holds newcomers back once it has lost a race for the mutex. The
/// release that wakes it is not enough: the first newcomer still takes the free mutex ahead of it,
/// however long it has waited, and it is by finding the mutex taken again that it starts to hold
/// newcomers back. From then on a free mutex is not taken by `try_lock`, nor by a `lock` that has
/// not waited yet, which waits behind the taker; the taker itself takes the free mutex, and
/// newcomers may take it again once it has.
#[test]
fn a_waiter_that_waited_long_holds_newcomers_back_from_a_mutex() {
    let mutex = Mutex::new(());
    let holder = ready(mutex.lock());
    let mut waiter = Box::pin(mutex.lock());
    assert!(poll_once(&mut waiter).is_pending());
    thread::sleep(PATIENCE);
    // The release wakes the waiter, which has not run yet when the first newcomer comes.
    drop(holder);
    let barging = ready(mutex.lock());

    // The waiter finds the mutex taken, having waited for long: it holds newcomers back from now
    // on.
    assert!(poll_once(&mut waiter).is_pending());
    drop(barging);
    assert!(mutex.try_lock().is_none());
    let mut newcomer = Box::pin(mutex.lock());
    assert!(poll_once(&mut newcomer).is_pending());

    let Poll::Ready(guard) = poll_once(&mut waiter) else {
        panic!("the release of the barging guard lets the starved waiter in");
    };
    drop(guard);
    assert!(poll_once(&mut newcomer).is_ready());
    assert!(mutex.try_lock().is_some());
}

/// A taker that holds newcomers back stops doing so when its `lock` future is dropped: the mutex
/// that was free, and yet not taken by a newcomer, is taken by one again, with no release needed.
#[test]
fn a_starved_lock_future_dropped_lets_newcomers_take_the_mutex_again() {
    let mutex = Mutex::new(());
    let holder = ready(mutex.lock());
    let mut waiter = Box::pin(mutex.lock());
    assert!(poll_once(&mut waiter).is_pending());
    thread::sleep(PATIENCE);
    drop(holder);
    let barging = ready(mutex.lock());
    assert!(poll_once(&mut waiter).is_pending());
    drop(barging);
    assert!(mutex.try_lock().is_none());

    drop(waiter);

    assert!(mutex.try_lock().is_some());
}

/// A taker that holds newcomers back, dropped after the release woke it, passes the wake-up on to
/// the newcomer it held back, which waits behind it: that release is the only one there will be, so
/// the newcomer would otherwise wait for good for a mutex that nobody holds.
#[test]
fn a_starved_lock_future_dropped_after_its_wake_wakes_the_newcomer_behind_it() {
    let mutex = Mutex::new(());
    let holder = ready(mutex.lock());
    let mut waiter = Box::pin(mutex.lock());
    assert!(poll_once(&mut waiter).is_pending());
    thread::sleep(PATIENCE);
    drop(holder);
    let barging = ready(mutex.lock());
    assert!(poll_once(&mut waiter).is_pending());
    let mut newcomer = Box::pin(mutex.lock());
    assert!(poll_once(&mut newcomer).is_pending());

    // The release notifies the waiter, which is then dropped without ever polling that
    // notification: it has to reach the newcomer instead.
    drop(barging);
    drop(waiter);
    assert!(poll_once(&mut newcomer).is_ready());
}

/// A taker that holds newcomers back does not hold back those that were waiting already: one that
/// waited from before it lost its race is notified ahead of it, as the starved taker listened
/// again behind it, and takes the free mutex, which `try_lock` still cannot. The starved taker gets
/// the mutex at the release that follows.
#[test]
fn a_taker_waiting_already_takes_the_mutex_ahead_of_a_starved_one() {
    let mutex = Mutex::new(());
    let holder = ready(mutex.lock());
    let mut starved = Box::pin(mutex.lock());
    assert!(poll_once(&mut starved).is_pending());
    let mut waiting = Box::pin(mutex.lock());
    assert!(poll_once(&mut waiting).is_pending());
    thread::sleep(PATIENCE);
    drop(holder);
    let barging = ready(mutex.lock());
    assert!(poll_once(&mut starved).is_pending());

    drop(barging);
    assert!(mutex.try_lock().is_none());
    let Poll::Ready(guard) = poll_once(&mut waiting) else {
        panic!("a taker that was waiting already takes the free mutex");
    };
    assert!(poll_once(&mut starved).is_pending());
    drop(guard);
    assert!(poll_once(&mut starved).is_ready());
}

/// `try_lock` takes a mutex that is free and fails, without waiting, on one that is held, whether
/// a `try_lock` or a `lock` holds it; it takes the mutex again once that guard is gone.
#[test]
fn try_lock_takes_a_free_mutex_and_fails_on_a_held_one() {
    let mutex = Mutex::new(1);

    let mut guard = mutex.try_lock().expect("a new mutex is free");
    *guard = 2;
    assert!(mutex.try_lock().is_none());
    drop(guard);

    let guard = ready(mutex.lock());
    assert!(mutex.try_lock().is_none());
    drop(guard);

    assert_eq!(*mutex.try_lock().expect("a released mutex is free"), 2);
}

/// A guard of `lock_arc` holds the mutex through an `Arc` of its own rather than a borrow, so it
/// outlives the borrow of the `Arc` it was taken through: it goes to another thread, which changes
/// the value and lets go of the mutex there, and this thread then sees the change.
#[test]
fn a_mutex_arc_guard_outlives_its_borrow_and_goes_to_another_thread() {
    let mutex = Arc::new(Mutex::new(1));
    let mut guard = ready(mutex.lock_arc());
    assert!(mutex.try_lock().is_none());

    thread::spawn(move || *guard = 2).join().unwrap();

    assert_eq!(
        *mutex
            .try_lock()
            .expect("the other thread let go of the mutex"),
        2
    );
}

/// `try_lock_arc` fails, without waiting, on a mutex that is held, whether a borrowed guard or an
/// `Arc` guard holds it, and takes it once that guard is gone; a borrowed `try_lock` fails while
/// the `Arc` guard holds it in turn.
#[test]
fn try_lock_arc_fails_on_a_held_mutex_and_takes_a_free_one() {
    let mutex = Arc::new(Mutex::new(1));

    let guard = ready(mutex.lock());
    assert!(mutex.try_lock_arc().is_none());
    drop(guard);

    let mut guard = mutex.try_lock_arc().expect("a released mutex is free");
    *guard = 2;
    assert!(mutex.try_lock_arc().is_none());
    assert!(mutex.try_lock().is_none());
    drop(guard);

    assert_eq!(*mutex.try_lock_arc().expect("a released mutex is free"), 2);
}

/// Dropping an `Arc` guard releases the mutex and wakes a task waiting for it, as dropping a
/// borrowed guard does: a `lock` and a `lock_arc` that wait while it is held each get in, the
/// first at its release and the second at the release of the guard the first took.
#[test]
fn dropping_a_mutex_arc_guard_lets_a_waiter_in() {
    let mutex = Arc::new(Mutex::new(()));
    let guard = ready(mutex.lock_arc());
    let mut borrowing = Box::pin(mutex.lock());
    assert!(poll_once(&mut borrowing).is_pending());
    let mut owning = Box::pin(mutex.lock_arc());
    assert!(poll_once(&mut owning).is_pending());

    drop(guard);
    let Poll::Ready(guard) = poll_once(&mut borrowing) else {
        panic!("the release of the Arc guard lets the first waiter in");
    };
    assert!(poll_once(&mut owning).is_pending());
    drop(guard);

    assert!(poll_once(&mut owning).is_ready());
}

/// The calls that hand out `Arc` guards are served as the borrowing ones are: a `lock_arc`
/// newcomer takes a free mutex ahead of a waiter that has not waited for long, and a `lock_arc`
/// that has waited for long, once it has lost such a race, holds newcomers back, `try_lock_arc`
/// and a `lock_arc` that has not waited yet alike, until it has had its turn.
#[test]
fn a_lock_arc_that_waited_long_holds_newcomers_back() {
    let mutex = Arc::new(Mutex::new(()));
    let holder = ready(mutex.lock_arc());
    let mut waiter = Box::pin(mutex.lock_arc());
    assert!(poll_once(&mut waiter).is_pending());
    thread::sleep(PATIENCE);
    // The release wakes the waiter, which has not run yet when the first newcomer comes.
    drop(holder);
    let barging = mutex
        .try_lock_arc()
        .expect("a free mutex is taken, whoever waits for it");

    // The waiter finds the mutex taken, having waited for long: it holds newcomers back from now
    // on.
    assert!(poll_once(&mut waiter).is_pending());
    drop(barging);
    assert!(mutex.try_lock_arc().is_none());
    let mut newcomer = Box::pin(mutex.lock_arc());
    assert!(poll_once(&mut newcomer).is_pending());

    let Poll::Ready(guard) = poll_once(&mut waiter) else {
        panic!("the release of the barging guard lets the starved waiter in");
    };
    drop(guard);
    assert!(poll_once(&mut newcomer).is_ready());
}

/// Readers share the lock and keep a writer waiting until the last of them is gone; a writer in
/// turn keeps readers out until it lets go, and the readers then see what it wrote.
#[test]
fn readers_share_and_a_writer_excludes() {
    let lock = RwLock::new(1);
    let first = ready(lock.read());
    let second = ready(lock.read());
    let mut writer = Box::pin(lock.write());
    assert!(poll_once(&mut writer).is_pending());
    drop(first);
    drop(second);
    let Poll::Ready(mut guard) = poll_once(&mut writer) else {
        panic!("the last reader lets the writer in");
    };
    *guard = 2;
    let mut reader = Box::pin(lock.read());
    assert!(poll_once(&mut reader).is_pending());
    drop(guard);
    let Poll::Ready(guard) = poll_once(&mut reader) else {
        panic!("the writer's release lets readers in");
    };
    assert_eq!(*guard, 2);
}

/// A writer's release lets in every reader that waits for it, not just the first: readers share the
/// lock, so once the writer is gone nothing keeps any of them out.
#[test]
fn a_writer_release_lets_every_waiting_reader_in() {
    let lock = RwLock::new(());
    let writer = lock.try_write().expect("a new lock is free to write");
    let mut first = Box::pin(lock.read());
    let mut second = Box::pin(lock.read());
    assert!(poll_once(&mut first).is_pending());
    assert!(poll_once(&mut second).is_pending());

    drop(writer);

    assert!(poll_once(&mut first).is_ready());
    assert!(poll_once(&mut second).is_ready());
}

/// A writer waiting for readers to leave holds new readers back, so that readers coming one after
/// the other cannot keep it out for good. It is also why a task holding a read guard must not ask
/// for another: the second `read` waits behind the writer, which waits for the first guard.
#[test]
fn a_waiting_writer_holds_new_readers_back() {
    let lock = RwLock::new(());
    let reader = ready(lock.read());
    let mut writer = Box::pin(lock.write());
    assert!(poll_once(&mut writer).is_pending());
    let mut late_reader = Box::pin(lock.read());
    assert!(poll_once(&mut late_reader).is_pending());
    drop(reader);
    assert!(poll_once(&mut writer).is_ready());
}

/// A `write` future dropped after the release woke it, before it took the lock, passes the wake-up
/// on, so the writer behind it is let in and not left waiting for a release that never comes.
#[test]
fn a_dropped_write_future_does_not_strand_the_next_writer() {
    let lock = RwLock::new(());
    let guard = ready(lock.write());
    let mut first = Box::pin(lock.write());
    assert!(poll_once(&mut first).is_pending());
    let mut second = Box::pin(lock.write());
    assert!(poll_once(&mut second).is_pending());
    // The release notifies `first`, which is then dropped without ever polling that
    // notification: it has to reach `second` instead.
    drop(guard);
    drop(first);
    assert!(poll_once(&mut second).is_ready());
}

/// A writer given up stops holding readers back: one that asks afterwards is let in.
#[test]
fn a_cancelled_writer_lets_readers_in_again() {
    let lock = RwLock::new(());
    let reader = ready(lock.read());
    let mut writer = Box::pin(lock.write());
    assert!(poll_once(&mut writer).is_pending());
    drop(writer);
    assert!(poll_once(&mut Box::pin(lock.read())).is_ready());
    drop(reader);
}

/// A writer given up while readers already wait behind it wakes every one of them, not just the
/// first: they wait only for that writer, and the readers holding the lock would wake a writer, not
/// them, when they go.
#[test]
fn a_cancelled_writer_wakes_every_reader_it_held_back() {
    let lock = RwLock::new(());
    let reader = ready(lock.read());
    let mut writer = Box::pin(lock.write());
    assert!(poll_once(&mut writer).is_pending());
    let mut first = Box::pin(lock.read());
    let mut second = Box::pin(lock.read());
    assert!(poll_once(&mut first).is_pending());
    assert!(poll_once(&mut second).is_pending());

    drop(writer);

    assert!(poll_once(&mut first).is_ready());
    assert!(poll_once(&mut second).is_ready());
    drop(reader);
}

/// `try_read` and `try_write` each take a lock that is free. Readers let other readers in and keep
/// every writer out, and a writer keeps everyone out; none of them waits, and the lock is free for
/// both again once the guards are gone.
#[test]
fn try_read_and_try_write_take_a_free_lock_and_fail_on_a_held_one() {
    let lock = RwLock::new(1);

    drop(lock.try_read().expect("a new lock is free to read"));
    drop(lock.try_write().expect("a new lock is free to write"));

    let first = lock.try_read().expect("a free lock is read");
    let second = lock.try_read().expect("readers share the lock");
    assert!(lock.try_write().is_none());
    drop(first);
    assert!(lock.try_write().is_none());
    drop(second);

    let mut writer = lock.try_write().expect("the readers are gone");
    *writer = 2;
    assert!(lock.try_read().is_none());
    assert!(lock.try_write().is_none());
    drop(writer);

    assert_eq!(*lock.try_read().expect("the writer is gone"), 2);
}

/// `try_read` fails while a writer waits for the lock, whether readers hold it or nobody does yet:
/// a waiting writer holds readers back from `try_read` as it does from `read`, until it has had its
/// turn.
#[test]
fn try_read_fails_while_a_writer_waits() {
    let lock = RwLock::new(());
    let reader = lock.try_read().expect("a new lock is free to read");
    let mut writer = Box::pin(lock.write());
    assert!(poll_once(&mut writer).is_pending());

    assert!(lock.try_read().is_none());
    drop(reader);
    // Free now, but the writer it woke has not taken the lock yet.
    assert!(lock.try_read().is_none());
    let Poll::Ready(guard) = poll_once(&mut writer) else {
        panic!("the last reader lets the writer in");
    };
    drop(guard);

    assert!(lock.try_read().is_some());
}

/// `try_write` looks at who holds the lock and not at who waits for it, unless a writer has waited
/// for long (see the tests that follow): where the lock is free and a writer that has not waited
/// for long waits, it takes the lock ahead of that writer. The writer finds the lock taken and goes
/// back to waiting, and gets it once the guard `try_write` gave is dropped.
#[test]
fn try_write_takes_a_free_lock_while_a_writer_waits() {
    let lock = RwLock::new(1);
    let holder = ready(lock.write());
    let mut waiting = Box::pin(lock.write());
    assert!(poll_once(&mut waiting).is_pending());
    // The release wakes the waiting writer, which has not run yet when `try_write` comes in.
    drop(holder);

    let mut barging = lock
        .try_write()
        .expect("a free lock is taken, whoever waits for it");
    *barging = 2;

    assert!(poll_once(&mut waiting).is_pending());
    drop(barging);
    let Poll::Ready(guard) = poll_once(&mut waiting) else {
        panic!("the release of the barging guard lets the waiting writer in");
    };
    assert_eq!(*guard, 2);
}

/// A writer that has waited for long holds newcomer writers back once it has lost a race for the
/// lock, as a taker of a mutex does. The release that wakes it is not enough: the first newcomer
/// still takes the free lock ahead of it, and it is by finding the lock taken again that it starts
/// to hold newcomers back. From then on a free lock is not taken by `try_write`, nor by a `write`
/// that has not waited yet, which waits behind the writer; the writer itself takes the free lock,
/// and newcomers may take it again once it has.
#[test]
fn a_writer_that_waited_long_holds_newcomer_writers_back() {
    let lock = RwLock::new(());
    let holder = ready(lock.write());
    let mut waiter = Box::pin(lock.write());
    assert!(poll_once(&mut waiter).is_pending());
    thread::sleep(PATIENCE);
    // The release wakes the waiter, which has not run yet when the first newcomer comes.
    drop(holder);
    let barging = ready(lock.write());

    // The waiter finds the lock taken, having waited for long: it holds newcomers back from now on.
    assert!(poll_once(&mut waiter).is_pending());
    drop(barging);
    assert!(lock.try_write().is_none());
    let mut newcomer = Box::pin(lock.write());
    assert!(poll_once(&mut newcomer).is_pending());

    let Poll::Ready(guard) = poll_once(&mut waiter) else {
        panic!("the release of the barging guard lets the starved writer in");
    };
    drop(guard);
    assert!(poll_once(&mut newcomer).is_ready());
    assert!(lock.try_write().is_some());
}

/// A writer that holds newcomer writers back stops doing so when its `write` future is dropped:
/// the lock that was free, and yet not taken by a newcomer, is taken by one again, with no release
/// needed.
#[test]
fn a_starved_write_future_dropped_lets_newcomer_writers_in_again() {
    let lock = RwLock::new(());
    let holder = ready(lock.write());
    let mut waiter = Box::pin(lock.write());
    assert!(poll_once(&mut waiter).is_pending());
    thread::sleep(PATIENCE);
    drop(holder);
    let barging = ready(lock.write());
    assert!(poll_once(&mut waiter).is_pending());
    drop(barging);
    assert!(lock.try_write().is_none());

    drop(waiter);

    assert!(lock.try_write().is_some());
}

/// A reader that has waited for long is let in ahead of a writer that waits for the lock, which
/// holds back every reader that has not waited. The release of the writer that holds the lock wakes
/// the reader and the waiting writer; the reader, which runs first, finds the writer waiting and
/// has waited for long, so it enters, as no writer holds the lock. The writer waits until the
/// reader lets go, holding back the readers that come after, as it did before.
#[test]
fn a_reader_that_waited_long_enters_ahead_of_a_waiting_writer() {
    let lock = RwLock::new(());
    let first = ready(lock.write());
    let mut reader = Box::pin(lock.read());
    assert!(poll_once(&mut reader).is_pending());
    let mut writer = Box::pin(lock.write());
    assert!(poll_once(&mut writer).is_pending());
    thread::sleep(PATIENCE);

    drop(first);

    let Poll::Ready(guard) = poll_once(&mut reader) else {
        panic!("a reader that waited for long enters, though a writer waits for the lock");
    };
    assert!(poll_once(&mut writer).is_pending());
    assert!(lock.try_read().is_none());
    drop(guard);
    assert!(poll_once(&mut writer).is_ready());
}

/// A reader that has waited for long holds every writer back until it is in. The writer woken
/// beside it takes the lock first, as the reader has not found it taken yet; the reader then does,
/// and from then on no writer takes the lock once it is free, neither one that waits for it nor
/// `try_write`, until the reader has entered.
#[test]
fn a_starved_reader_holds_every_writer_back_until_it_is_in() {
    let lock = RwLock::new(());
    let first = ready(lock.write());
    let mut reader = Box::pin(lock.read());
    assert!(poll_once(&mut reader).is_pending());
    let mut second = Box::pin(lock.write());
    assert!(poll_once(&mut second).is_pending());
    thread::sleep(PATIENCE);
    drop(first);

    // The writer runs first, and takes the lock before the reader has found it taken.
    let Poll::Ready(second) = poll_once(&mut second) else {
        panic!("the release of the first writer lets the second in");
    };
    // The reader finds the lock taken, having waited for long: it holds the writers back from now
    // on.
    assert!(poll_once(&mut reader).is_pending());
    let mut third = Box::pin(lock.write());
    assert!(poll_once(&mut third).is_pending());
    drop(second);
    assert!(poll_once(&mut third).is_pending());
    assert!(lock.try_write().is_none());

    let Poll::Ready(guard) = poll_once(&mut reader) else {
        panic!("the release of the second writer lets the starved reader in");
    };
    drop(guard);
    assert!(poll_once(&mut third).is_ready());
}

/// A reader that holds every writer back stops doing so when its `read` future is dropped: the
/// writer it kept out of a free lock is woken and takes it. No reader took the lock, so none will
/// release it and wake a writer: the dropped future has to.
#[test]
fn a_starved_read_future_dropped_lets_the_writers_in() {
    let lock = RwLock::new(());
    let first = ready(lock.write());
    let mut reader = Box::pin(lock.read());
    assert!(poll_once(&mut reader).is_pending());
    let mut second = Box::pin(lock.write());
    assert!(poll_once(&mut second).is_pending());
    thread::sleep(PATIENCE);
    drop(first);
    let Poll::Ready(second) = poll_once(&mut second) else {
        panic!("the release of the first writer lets the second in");
    };
    assert!(poll_once(&mut reader).is_pending());
    let mut third = Box::pin(lock.write());
    assert!(poll_once(&mut third).is_pending());
    drop(second);
    assert!(poll_once(&mut third).is_pending());

    drop(reader);

    assert!(poll_once(&mut third).is_ready());
}

/// A starved reader goes ahead of a starved writer: the writer holds newcomer writers back, but
/// the reader holds every writer back, starved or not, until it is in. Readers that are not
/// starved are held back by the waiting writer as before, and the writer gets the lock once the
/// reader lets go.
#[test]
fn a_starved_reader_goes_ahead_of_a_starved_writer() {
    let lock = RwLock::new(());
    let first = ready(lock.write());
    let mut reader = Box::pin(lock.read());
    assert!(poll_once(&mut reader).is_pending());
    let mut writer = Box::pin(lock.write());
    assert!(poll_once(&mut writer).is_pending());
    thread::sleep(PATIENCE);
    drop(first);
    let barging = lock
        .try_write()
        .expect("a free lock is taken, whoever waits for it");
    // Both find the lock taken, having waited for long: both starve.
    assert!(poll_once(&mut writer).is_pending());
    assert!(poll_once(&mut reader).is_pending());

    drop(barging);
    assert!(lock.try_write().is_none());
    assert!(poll_once(&mut writer).is_pending());
    let Poll::Ready(guard) = poll_once(&mut reader) else {
        panic!("the starved reader enters ahead of the starved writer");
    };
    assert!(lock.try_read().is_none());
    drop(guard);
    assert!(poll_once(&mut writer).is_ready());
}

/// Guards of `read_arc` and `write_arc` hold the lock through an `Arc` of their own rather than a
/// borrow, so they outlive the borrow of the `Arc` they were taken through: a write guard goes to
/// another thread, which changes the value and lets go of the lock there, and read guards go to
/// two threads at once, which both see the change.
#[test]
fn rwlock_arc_guards_outlive_their_borrow_and_go_to_other_threads() {
    let lock = Arc::new(RwLock::new(1));
    let mut writer = ready(lock.write_arc());
    assert!(lock.try_read().is_none());
    thread::spawn(move || *writer = 2).join().unwrap();

    let readers: Vec<_> = (0..2)
        .map(|_| {
            let reader = ready(lock.read_arc());
            thread::spawn(move || *reader)
        })
        .collect();
    for reader in readers {
        assert_eq!(reader.join().unwrap(), 2);
    }

    assert!(lock.try_write().is_some());
}

/// `read_arc` guards share the lock, with each other and with borrowed read guards, and keep a
/// `write_arc` waiting until the last of them is gone; the write guard in turn keeps readers of
/// either kind out until it lets go, and they then see what it wrote.
#[test]
fn read_arc_guards_share_and_a_write_arc_guard_excludes() {
    let lock = Arc::new(RwLock::new(1));
    let first = ready(lock.read_arc());
    let second = ready(lock.read());
    let third = ready(lock.read_arc());
    let mut writer = Box::pin(lock.write_arc());
    assert!(poll_once(&mut writer).is_pending());
    drop(first);
    drop(second);
    assert!(poll_once(&mut writer).is_pending());
    drop(third);
    let Poll::Ready(mut guard) = poll_once(&mut writer) else {
        panic!("the last reader lets the writer in");
    };
    *guard = 2;

    let mut owning = Box::pin(lock.read_arc());
    assert!(poll_once(&mut owning).is_pending());
    let mut borrowing = Box::pin(lock.read());
    assert!(poll_once(&mut borrowing).is_pending());
    drop(guard);
    let Poll::Ready(owning) = poll_once(&mut owning) else {
        panic!("the writer's release lets readers in");
    };
    let Poll::Ready(borrowing) = poll_once(&mut borrowing) else {
        panic!("the writer's release lets every reader in");
    };
    assert_eq!((*owning, *borrowing), (2, 2));
}

/// `try_read_arc` and `try_write_arc` each take a lock that is free, and fail, without waiting,
/// wherever `try_read` and `try_write` would: readers let other readers in and keep every writer
/// out, a writer keeps everyone out, and a waiting writer keeps new readers out.
#[test]
fn try_read_arc_and_try_write_arc_take_a_free_lock_and_fail_on_a_held_one() {
    let lock = Arc::new(RwLock::new(1));

    let first = lock.try_read_arc().expect("a new lock is free to read");
    let second = lock.try_read_arc().expect("readers share the lock");
    assert!(lock.try_write_arc().is_none());
    let mut writer = Box::pin(lock.write());
    assert!(poll_once(&mut writer).is_pending());
    assert!(lock.try_read_arc().is_none());
    drop(writer);
    drop(first);
    assert!(lock.try_write_arc().is_none());
    drop(second);

    let mut writer = lock.try_write_arc().expect("the readers are gone");
    *writer = 2;
    assert!(lock.try_read_arc().is_none());
    assert!(lock.try_write_arc().is_none());
    assert!(lock.try_read().is_none());
    drop(writer);

    assert_eq!(*lock.try_read_arc().expect("the writer is gone"), 2);
}

/// A waiting `write_arc` holds new readers back, `read_arc` among them, as a waiting `write` does,
/// and once it has waited for long and lost a race for the lock, it holds newcomer writers back
/// too, `try_write_arc` and a `write_arc` that has not waited yet alike, until it has had its turn.
#[test]
fn a_write_arc_holds_readers_back_and_once_it_waited_long_newcomer_writers() {
    let lock = Arc::new(RwLock::new(()));
    let reader = ready(lock.read_arc());
    let mut waiter = Box::pin(lock.write_arc());
    assert!(poll_once(&mut waiter).is_pending());
    let mut late_reader = Box::pin(lock.read_arc());
    assert!(poll_once(&mut late_reader).is_pending());
    assert!(lock.try_read_arc().is_none());
    // Given up before it has waited for long, so that no reader is let in ahead of the writers.
    drop(late_reader);
    thread::sleep(PATIENCE);
    // The release wakes the waiter, which has not run yet when the first newcomer comes.
    drop(reader);
    let barging = lock
        .try_write_arc()
        .expect("a free lock is taken, whoever waits for it");

    // The waiter finds the lock taken, having waited for long: it holds newcomers back from now on.
    assert!(poll_once(&mut waiter).is_pending());
    drop(barging);
    assert!(lock.try_write_arc().is_none());
    let mut newcomer = Box::pin(lock.write_arc());
    assert!(poll_once(&mut newcomer).is_pending());

    let Poll::Ready(guard) = poll_once(&mut waiter) else {
        panic!("the release of the barging guard lets the starved writer in");
    };
    drop(guard);
    assert!(poll_once(&mut newcomer).is_ready());
}

/// A panic while a guard of a mutex is held drops the guard on the way out, which releases the
/// mutex: nothing poisons it, and the next holder finds the value as the panicking code left it.
#[test]
fn a_panic_while_a_mutex_guard_is_held_releases_the_mutex() {
    let mutex = Mutex::new(1);

    // The guard is taken outside the closure, which then owns it: a `ready` that panicked inside
    // would be caught along with the panic this test is after.
    let mut guard = ready(mutex.lock());
    let panicked = catch_unwind(AssertUnwindSafe(move || {
        *guard = 2;
        panic!("a panic while the guard is held");
    }));

    assert!(panicked.is_err());
    assert_eq!(*mutex.try_lock().expect("the panic released the mutex"), 2);
}

/// A panic while a write guard or a read guard is held drops the guard on the way out, which
/// releases the lock: nothing poisons it, the next holder finds the value as the panicking code
/// left it, and the reader that panicked is not left counted among the readers keeping writers out.
#[test]
fn a_panic_while_an_rwlock_guard_is_held_releases_the_lock() {
    let lock = RwLock::new(1);

    // Each guard is taken outside its closure, which then owns it, as in the mutex's test above.
    let mut guard = ready(lock.write());
    let panicked = catch_unwind(AssertUnwindSafe(move || {
        *guard = 2;
        panic!("a panic while the write guard is held");
    }));
    assert!(panicked.is_err());
    assert_eq!(*lock.try_read().expect("the panic released the lock"), 2);

    let guard = ready(lock.read());
    let panicked = catch_unwind(AssertUnwindSafe(move || {
        let _guard = guard;
        panic!("a panic while a read guard is held");
    }));
    assert!(panicked.is_err());
    *lock.try_write().expect("the panic released the lock") = 3;
    assert_eq!(*lock.try_read().expect("the writer is gone"), 3);
}

/// A lock borrowed mutably has no guard to wait for, so `get_mut` reaches the value without a wait,
/// and what it changes is what the next guard sees.
#[test]
fn get_mut_reaches_the_value_without_a_guard() {
    let mut mutex = Mutex::new(vec![1]);
    mutex.get_mut().push(2);
    assert_eq!(*ready(mutex.lock()), [1, 2]);

    let mut lock = RwLock::new(vec![1]);
    lock.get_mut().push(2);
    assert_eq!(*ready(lock.read()), [1, 2]);
}

/// `into_inner` hands back the value as the guards left it, with no wait: the lock is moved, so
/// none is left to hold it.
#[test]
fn into_inner_hands_back_the_value() {
    let mutex = Mutex::new(vec![1]);
    ready(mutex.lock()).push(2);
    assert_eq!(mutex.into_inner(), [1, 2]);

    let lock = RwLock::new(vec![1]);
    ready(lock.write()).push(2);
    assert_eq!(lock.into_inner(), [1, 2]);
}

/// A lock made by `default` holds the default of its value type, and one made by `from` holds the
/// value it is given; a lock made either way is free.
#[test]
fn a_lock_is_made_from_a_default_or_from_a_value() {
    let mutex = Mutex::<Vec<u8>>::default();
    assert!(mutex.try_lock().expect("a new mutex is free").is_empty());
    let mutex = Mutex::from(7);
    assert_eq!(*mutex.try_lock().expect("a new mutex is free"), 7);

    let lock = RwLock::<Vec<u8>>::default();
    assert!(lock.try_write().expect("a new lock is free").is_empty());
    let lock = RwLock::from(7);
    assert_eq!(*lock.try_write().expect("a new lock is free"), 7);
}

/// A lock prints its value while that can be read at once, and `<locked>` while it cannot: for a
/// mutex, while it is held and also while it is free but a taker that has waited for long holds
/// newcomers back, which holds the printing back as it holds `try_lock` back; for an `RwLock`,
/// while a writer holds it and also while a writer only waits for it, which holds the printing back
/// as it holds any reader back. Readers holding the `RwLock` do not keep it from printing its
/// value.
#[test]
fn a_lock_prints_its_value_while_free_and_a_placeholder_while_held() {
    let mutex = Mutex::new(1);
    assert_eq!(format!("{mutex:?}"), "Mutex { value: 1 }");
    let guard = ready(mutex.lock());
    assert_eq!(format!("{mutex:?}"), "Mutex { value: <locked> }");
    drop(guard);
    assert_eq!(format!("{mutex:?}"), "Mutex { value: 1 }");
    let holder = ready(mutex.lock());
    let mut waiter = Box::pin(mutex.lock());
    assert!(poll_once(&mut waiter).is_pending());
    thread::sleep(PATIENCE);
    drop(holder);
    // The waiter finds the mutex taken by a newcomer, having waited for long, and then the mutex
    // is free but not for newcomers.
    let barging = ready(mutex.lock());
    assert!(poll_once(&mut waiter).is_pending());
    drop(barging);
    assert_eq!(format!("{mutex:?}"), "Mutex { value: <locked> }");
    drop(waiter);
    assert_eq!(format!("{mutex:?}"), "Mutex { value: 1 }");

    let lock = RwLock::new(1);
    assert_eq!(format!("{lock:?}"), "RwLock { value: 1 }");
    let writer = ready(lock.write());
    assert_eq!(format!("{lock:?}"), "RwLock { value: <locked> }");
    drop(writer);
    let reader = ready(lock.read());
    assert_eq!(format!("{lock:?}"), "RwLock { value: 1 }");
    let mut waiting = Box::pin(lock.write());
    assert!(poll_once(&mut waiting).is_pending());
    assert_eq!(format!("{lock:?}"), "RwLock { value: <locked> }");
    drop(waiting);
    assert_eq!(format!("{lock:?}"), "RwLock { value: 1 }");
    drop(reader);
}

/// Each guard prints as the value it stands for, with the options of the format it is printed
/// with passed on to the value: a width, or the pretty form.
#[test]
fn a_guard_prints_as_its_value() {
    let mutex = Mutex::new(1);
    let guard = ready(mutex.lock());
    assert_eq!(format!("{guard:?}"), "1");
    assert_eq!(format!("{guard:>3?}"), "  1");
    drop(guard);

    let lock = RwLock::new(vec![1, 2]);
    let reader = ready(lock.read());
    assert_eq!(format!("{reader:?}"), "[1, 2]");
    assert_eq!(format!("{reader:#?}"), "[\n    1,\n    2,\n]");
    drop(reader);
    let writer = ready(lock.write());
    assert_eq!(format!("{writer:?}"), "[1, 2]");
    assert_eq!(format!("{writer:#?}"), "[\n    1,\n    2,\n]");
}

/// An `Arc` guard holds a clone of the `Arc` it was taken through, whichever call made it, and
/// lets go of that clone as it is dropped: the lock lives on in its guard once every other handle
/// to it is gone, and the guard still reaches the value.
#[test]
fn an_arc_guard_holds_a_clone_of_the_arc_until_dropped() {
    let mutex = Arc::new(Mutex::new(vec![1]));
    let guard = ready(mutex.lock_arc());
    assert_eq!(Arc::strong_count(&mutex), 2);
    drop(guard);
    assert_eq!(Arc::strong_count(&mutex), 1);
    let mut guard = mutex.try_lock_arc().expect("a released mutex is free");
    assert_eq!(Arc::strong_count(&mutex), 2);
    drop(mutex);
    guard.push(2);
    assert_eq!(*guard, [1, 2]);

    let lock = Arc::new(RwLock::new(vec![1]));
    let first = ready(lock.read_arc());
    let second = lock.try_read_arc().expect("readers share the lock");
    assert_eq!(Arc::strong_count(&lock), 3);
    drop(first);
    drop(second);
    assert_eq!(Arc::strong_count(&lock), 1);
    let writer = ready(lock.write_arc());
    assert_eq!(Arc::strong_count(&lock), 2);
    drop(writer);
    let mut writer = lock.try_write_arc().expect("the writer is gone");
    assert_eq!(Arc::strong_count(&lock), 2);
    drop(lock);
    writer.push(2);
    assert_eq!(*writer, [1, 2]);
}

/// A lock of an unsized value, behind an `Arc`, hands out `Arc` guards of that value: of a slice
/// for a mutex, and of a trait object for a readers-writer lock.
#[test]
fn arc_guards_reach_an_unsized_value() {
    let mutex: Arc<Mutex<[u8]>> = Arc::new(Mutex::new([1, 2, 3]));
    ready(mutex.lock_arc())[0] = 4;
    assert_eq!(
        *mutex.try_lock_arc().expect("a released mutex is free"),
        [4, 2, 3]
    );

    let lock: Arc<RwLock<dyn fmt::Debug + Send + Sync>> = Arc::new(RwLock::new(5u8));
    assert_eq!(format!("{:?}", &*ready(lock.read_arc())), "5");
    assert_eq!(format!("{:?}", &*ready(lock.write_arc())), "5");
}

/// Each `Arc` guard prints as the value it stands for, as a borrowed guard does, with the options
/// of the format it is printed with passed on to the value.
#[test]
fn an_arc_guard_prints_as_its_value() {
    let mutex = Arc::new(Mutex::new(1));
    let guard = ready(mutex.lock_arc());
    assert_eq!(format!("{guard:?}"), "1");
    assert_eq!(format!("{guard:>3?}"), "  1");
    drop(guard);

    let lock = Arc::new(RwLock::new(vec![1, 2]));
    let reader = ready(lock.read_arc());
    assert_eq!(format!("{reader:?}"), "[1, 2]");
    assert_eq!(format!("{reader:#?}"), "[\n    1,\n    2,\n]");
    drop(reader);
    let writer = ready(lock.write_arc());
    assert_eq!(format!("{writer:?}"), "[1, 2]");
    assert_eq!(format!("{writer:#?}"), "[\n    1,\n    2,\n]");
}

/// A mutex behind an `Arc` coerces to one of a trait object, which is what the value being the
/// last field of the lock lets it do.
#[test]
fn a_mutex_coerces_to_an_unsized_value() {
    let mutex: Arc<Mutex<dyn fmt::Debug + Send>> = Arc::new(Mutex::new(5u8));
    let guard = ready(mutex.lock());
    assert_eq!(format!("{:?}", &*guard), "5");
}

/// A readers-writer lock behind an `Arc` coerces to one of a trait object, as a mutex does.
#[test]
fn a_rwlock_coerces_to_an_unsized_value() {
    let lock: Arc<RwLock<dyn fmt::Debug + Send + Sync>> = Arc::new(RwLock::new(5u8));
    let guard = ready(lock.read());
    assert_eq!(format!("{:?}", &*guard), "5");
}

/// Threads that take a mutex over and over, each adding one to a count that it reads and writes
/// back as two steps: a second holder let in at the same time would lose a step of the count, and a
/// release that no waiting thread heard of would leave a thread waiting for good, which the timeout
/// would catch.
#[test]
#[timeout(15000)]
fn a_mutex_admits_one_holder_at_a_time_across_threads() {
    let (threads, rounds) = if cfg!(miri) { (3, 20) } else { (8, 1000) };
    let mutex = Arc::new(Mutex::new(0usize));
    let start = Arc::new(Barrier::new(threads));

    let takers: Vec<_> = (0..threads)
        .map(|_| {
            let mutex = mutex.clone();
            let start = start.clone();
            thread::spawn(move || {
                // Lets the threads in together, so that they contend from their first round rather
                // than each running its rounds before the next is spawned.
                start.wait();
                for _ in 0..rounds {
                    let mut count = block_on(mutex.lock());
                    let seen = *count;
                    // Gives a second holder, were one let in, a moment to come in between.
                    linger();
                    *count = seen + 1;
                }
            })
        })
        .collect();
    for taker in takers {
        taker.join().unwrap();
    }

    assert_eq!(*block_on(mutex.lock()), threads * rounds);
}

/// Writers that update a pair in two steps, and readers that check the two halves agree, all on
/// threads of their own: a reader let in while a writer is halfway through would see them differ,
/// and two writers let in together would lose a step. Readers held back by a waiting writer, and
/// writers woken by the last reader to leave, must each get through, which the timeout would catch.
#[test]
#[timeout(15000)]
fn readers_never_see_a_pair_half_updated_by_a_writer() {
    let (writers, readers, rounds) = if cfg!(miri) { (2, 2, 10) } else { (4, 4, 500) };
    let lock = Arc::new(RwLock::new((0usize, 0usize)));
    let start = Arc::new(Barrier::new(writers + readers));

    let writer_threads: Vec<_> = (0..writers)
        .map(|_| {
            let lock = lock.clone();
            let start = start.clone();
            thread::spawn(move || {
                // Lets the threads in together, so that they contend from their first round rather
                // than each running its rounds before the next is spawned.
                start.wait();
                for _ in 0..rounds {
                    let mut guard = block_on(lock.write());
                    guard.0 += 1;
                    // Gives a reader, were one let in, a moment to look in between.
                    linger();
                    guard.1 += 1;
                }
            })
        })
        .collect();
    let reader_threads: Vec<_> = (0..readers)
        .map(|_| {
            let lock = lock.clone();
            let start = start.clone();
            thread::spawn(move || {
                // Lets the threads in together, as the writers do.
                start.wait();
                for _ in 0..rounds {
                    let guard = block_on(lock.read());
                    assert_eq!(guard.0, guard.1);
                }
            })
        })
        .collect();
    for thread in writer_threads.into_iter().chain(reader_threads) {
        thread.join().unwrap();
    }

    assert_eq!(*block_on(lock.read()), (writers * rounds, writers * rounds));
}

/// A thread that waits for a mutex is let in, though two others keep taking it: each takes it again
/// as soon as it has let go, with a moment of holding it in between, so that the mutex is free for
/// an instant at a time. The waiter gets in either by finding it free in that instant or, once it
/// has waited for long and lost a race, by holding the two back until it has the mutex; did it
/// never get in, the timeout would catch it. No waiter is left counted as holding newcomers back
/// when they are done, or the mutex would not be taken afterwards.
#[test]
#[timeout(15000)]
fn a_mutex_that_two_threads_keep_taking_still_lets_a_third_in() {
    let mutex = Arc::new(Mutex::new(()));
    let stop = Arc::new(AtomicBool::new(false));
    let start = Arc::new(Barrier::new(3));

    let takers: Vec<_> = (0..2)
        .map(|_| {
            let mutex = mutex.clone();
            let stop = stop.clone();
            let start = start.clone();
            thread::spawn(move || {
                // Lets the threads in together, so that the waiter meets two takers from the start.
                start.wait();
                while !stop.load(Ordering::Relaxed) {
                    let guard = block_on(mutex.lock());
                    linger();
                    drop(guard);
                }
            })
        })
        .collect();
    let waiter = thread::spawn({
        let mutex = mutex.clone();
        let stop = stop.clone();
        move || {
            start.wait();
            let guard = block_on(mutex.lock());
            // Lets the takers know that this thread has had its turn.
            stop.store(true, Ordering::Relaxed);
            drop(guard);
        }
    });
    waiter.join().unwrap();
    for taker in takers {
        taker.join().unwrap();
    }

    assert!(mutex.try_lock().is_some());
}

/// A reader that waits for a lock is let in, though two writers keep taking it: each takes it again
/// as soon as it has let go, with a moment of holding it in between, so that a writer nearly always
/// holds or waits for the lock, which keeps out a reader that has not waited for long. The reader
/// gets in either by finding no writer holding or waiting in that instant or, once it has waited
/// for long, by being let in ahead of the writers, which then wait for it; did it never get in, the
/// timeout would catch it. No waiter is left counted as holding anyone back when they are done, or
/// the lock would not be taken afterwards.
#[test]
#[timeout(15000)]
fn an_rwlock_that_two_writers_keep_taking_still_lets_a_reader_in() {
    let lock = Arc::new(RwLock::new(()));
    let stop = Arc::new(AtomicBool::new(false));
    let start = Arc::new(Barrier::new(3));

    let writers: Vec<_> = (0..2)
        .map(|_| {
            let lock = lock.clone();
            let stop = stop.clone();
            let start = start.clone();
            thread::spawn(move || {
                // Lets the threads in together, so that the reader meets two writers from the
                // start.
                start.wait();
                while !stop.load(Ordering::Relaxed) {
                    let guard = block_on(lock.write());
                    linger();
                    drop(guard);
                }
            })
        })
        .collect();
    let reader = thread::spawn({
        let lock = lock.clone();
        let stop = stop.clone();
        move || {
            start.wait();
            let guard = block_on(lock.read());
            // Lets the writers know that this thread has had its turn.
            stop.store(true, Ordering::Relaxed);
            drop(guard);
        }
    });
    reader.join().unwrap();
    for writer in writers {
        writer.join().unwrap();
    }

    assert!(lock.try_write().is_some());
    assert!(lock.try_read().is_some());
}

/// A `lock` that starts to wait just as the holder lets go is let in, wherever the release falls
/// among its steps: before its first try, between that try and its listener, or after both. Only
/// that one release can let the waiter in, so a release it misses leaves it waiting for good, which
/// the timeout catches.
#[test]
#[timeout(15000)]
fn a_release_racing_a_lock_is_not_missed() {
    let rounds = if cfg!(miri) { 20 } else { 20_000 };
    handoff(
        Arc::new(Mutex::new(())),
        rounds,
        |mutex, start_waiting| {
            let _guard = mutex
                .try_lock()
                .expect("nothing holds the mutex between rounds");
            start_waiting();
        },
        |mutex| drop(block_on(mutex.lock())),
    );
}

/// A `read` that starts to wait just as the writer lets go is let in, wherever the release falls
/// among its steps: before its first try, between that try and its listener, or after both. Only
/// that one release can let the reader in, so a release it misses leaves it waiting for good, which
/// the timeout catches.
#[test]
#[timeout(15000)]
fn a_write_release_racing_a_read_is_not_missed() {
    let rounds = if cfg!(miri) { 20 } else { 20_000 };
    handoff(
        Arc::new(RwLock::new(())),
        rounds,
        |lock, start_waiting| {
            let _guard = lock
                .try_write()
                .expect("nothing holds the lock between rounds");
            start_waiting();
        },
        |lock| drop(block_on(lock.read())),
    );
}

/// A `write` that starts to wait just as the reader lets go is let in, wherever the release falls
/// among its steps: before its first try, between that try and its listener, or after both. Only
/// that one release can let the writer in, so a release it misses leaves it waiting for good, which
/// the timeout catches.
#[test]
#[timeout(15000)]
fn a_read_release_racing_a_write_is_not_missed() {
    let rounds = if cfg!(miri) { 20 } else { 20_000 };
    handoff(
        Arc::new(RwLock::new(())),
        rounds,
        |lock, start_waiting| {
            let _guard = lock
                .try_read()
                .expect("nothing holds the lock between rounds");
            start_waiting();
        },
        |lock| drop(block_on(lock.write())),
    );
}

/// A `lock` that starts to wait just as a starved taker's `lock` future is dropped is let in. The
/// mutex is free, and its last release went to the starved taker, which passes it on as it is
/// dropped, before it stops holding newcomers back: a newcomer that listens in between is held back
/// from the free mutex, and only the starved taker's going can wake it. The drop is spread across
/// the steps of the newcomer's wait, so that some rounds fall in between, and a newcomer that
/// nothing wakes waits for good, which the timeout catches.
#[test]
#[timeout(15000)]
fn a_starved_lock_future_dropped_racing_a_lock_does_not_strand_it() {
    let rounds = if cfg!(miri) { 20 } else { 2_000 };
    handoff(
        Arc::new(Mutex::new(())),
        rounds,
        |mutex, start_waiting| {
            let starved = starved_lock(mutex);
            start_waiting();
            drop(starved);
        },
        |mutex| drop(block_on(mutex.lock())),
    );
}

/// A `write` that starts to wait just as a starved writer's `write` future is dropped is let in,
/// as a `lock` is when a starved taker of a mutex goes (see the test above).
#[test]
#[timeout(15000)]
fn a_starved_write_future_dropped_racing_a_write_does_not_strand_it() {
    let rounds = if cfg!(miri) { 20 } else { 2_000 };
    handoff(
        Arc::new(RwLock::new(())),
        rounds,
        |lock, start_waiting| {
            let starved = starved_write(lock);
            start_waiting();
            drop(starved);
        },
        |lock| drop(block_on(lock.write())),
    );
}

/// Wakers that come back into the lock from their wake, each taking it and letting go at once or
/// listening for it and giving up, while threads take and release it: nothing deadlocks, and the
/// lock still admits one holder at a time.
///
/// A release notifies the event that waiting threads listen to, and the notification runs their
/// wakers, which come back into that very event from the thread that is notifying it: were the
/// event to run them with its own state still held, that thread would wait for itself for good,
/// which the timeout would catch. The count is read and written back as two steps, so a second
/// holder let in at the same time would lose a step of it.
#[test]
#[timeout(15000)]
fn wakers_coming_back_into_the_lock_from_many_threads_do_not_deadlock() {
    let (threads, rounds) = if cfg!(miri) { (3, 10) } else { (4, 250) };
    let mutex = Arc::new(Mutex::new(0usize));
    let start = Arc::new(Barrier::new(threads));

    let takers: Vec<_> = (0..threads)
        .map(|_| {
            let mutex = mutex.clone();
            let start = start.clone();
            thread::spawn(move || {
                // Lets the threads in together, so that they contend from their first round rather
                // than each running its rounds before the next is spawned.
                start.wait();
                for _ in 0..rounds {
                    let mut count = block_on(ComingBack {
                        mutex: mutex.clone(),
                        future: pin!(mutex.lock()),
                    });
                    let seen = *count;
                    // Gives a second holder, were one let in, a moment to come in between.
                    linger();
                    *count = seen + 1;
                }
            })
        })
        .collect();
    for taker in takers {
        taker.join().unwrap();
    }

    assert_eq!(*block_on(mutex.lock()), threads * rounds);
}

/// Polls `future` once, with a waker that goes nowhere.
fn poll_once<F>(future: &mut F) -> Poll<F::Output>
where
    F: Future + Unpin,
{
    pin!(future).poll(&mut Context::from_waker(Waker::noop()))
}

/// Polls `future` once, with a waker that goes nowhere, and hands back what it completed with.
///
/// Panics if the future is still pending: a test that takes a lock it expects to be free fails
/// here, rather than waits for a release that nothing in it will ever make.
fn ready<F>(future: F) -> F::Output
where
    F: Future,
{
    let Poll::Ready(output) = pin!(future).poll(&mut Context::from_waker(Waker::noop())) else {
        panic!("the future is pending, where it was expected to complete at once");
    };

    output
}

/// Polls `future` once with `waker`.
fn poll_with<F>(future: &mut F, waker: &Waker) -> Poll<F::Output>
where
    F: Future + Unpin,
{
    Pin::new(future).poll(&mut Context::from_waker(waker))
}

/// A waker that counts how often it is woken, and the count.
fn counting_waker() -> (Arc<AtomicUsize>, Waker) {
    let woken = Arc::new(AtomicUsize::new(0));
    let waker = Waker::from(Arc::new(CountWakes(woken.clone())));

    (woken, waker)
}

/// A `lock` future of `mutex` that holds newcomers back, on a mutex that is free and whose last
/// release notified it: the future has waited for long, and then found the mutex taken.
fn starved_lock(mutex: &Mutex<()>) -> Pin<Box<impl Future<Output = MutexGuard<'_, ()>>>> {
    let holder = mutex
        .try_lock()
        .expect("nothing holds the mutex between rounds");
    let mut waiter = Box::pin(mutex.lock());
    assert!(poll_once(&mut waiter).is_pending());
    thread::sleep(PATIENCE);
    drop(holder);
    let barging = mutex.try_lock().expect("the release left the mutex free");
    assert!(poll_once(&mut waiter).is_pending());
    drop(barging);
    assert!(
        mutex.try_lock().is_none(),
        "the waiter holds newcomers back"
    );

    waiter
}

/// A `write` future of `lock` that holds newcomer writers back, on a lock that is free and whose
/// last release notified it: the future has waited for long, and then found the lock taken.
fn starved_write(lock: &RwLock<()>) -> Pin<Box<impl Future<Output = RwLockWriteGuard<'_, ()>>>> {
    let holder = lock
        .try_write()
        .expect("nothing holds the lock between rounds");
    let mut waiter = Box::pin(lock.write());
    assert!(poll_once(&mut waiter).is_pending());
    thread::sleep(PATIENCE);
    drop(holder);
    let barging = lock.try_write().expect("the release left the lock free");
    assert!(poll_once(&mut waiter).is_pending());
    drop(barging);
    assert!(
        lock.try_write().is_none(),
        "the waiter holds newcomers back"
    );

    waiter
}

/// Lets a moment pass while a lock is held.
///
/// A spin rather than a yield: on a busy machine, a yield hands the thread away with the lock
/// held, keeping every other thread waiting for it until the thread gets its turn again.
fn linger() {
    for _ in 0..64 {
        hint::spin_loop();
    }
}

/// Runs `rounds` handoffs of `lock` between this thread and another. Each has this thread take the
/// lock in `hold`, which calls the function it is given to let the other thread start to `wait`
/// for the lock, and then returns, which releases the lock. The release comes a little later each
/// round, so that it is spread across the steps of the wait. Nothing else ever releases the lock,
/// so a release the waiter misses leaves it waiting for good, which the timeout of a test catches.
fn handoff<L, H, W>(lock: Arc<L>, rounds: usize, hold: H, wait: W)
where
    L: Send + Sync + 'static,
    H: Fn(&L, &dyn Fn()),
    W: Fn(&L) + Send + 'static,
{
    let barrier = Arc::new(Barrier::new(2));
    let waiter = thread::spawn({
        let lock = lock.clone();
        let barrier = barrier.clone();
        move || {
            for _ in 0..rounds {
                // Meets the holder as it is about to let go, and starts to wait.
                barrier.wait();
                wait(&lock);
                // The wait is over, so the next round may take the lock again.
                barrier.wait();
            }
        }
    });
    for round in 0..rounds {
        // The function passed to `hold` is where the waiter is met, and the lock is released as
        // `hold` returns, after a spin that is longer each round. Left to the timing of the two
        // threads alone, the release would seldom fall between the steps of the wait.
        hold(&lock, &|| {
            barrier.wait();
            for _ in 0..round % 256 {
                hint::spin_loop();
            }
        });
        // Waits for the waiter to have had its turn, so that the next round finds the lock free.
        barrier.wait();
    }
    waiter.join().unwrap();
}

/// A future that polls `future` through a waker that comes back into `mutex` from its wake, before
/// it wakes the task that polled this future.
struct ComingBack<F> {
    mutex: Arc<Mutex<usize>>,
    future: F,
}

impl<F> Future for ComingBack<F>
where
    F: Future + Unpin,
{
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<F::Output> {
        let this = self.get_mut();
        let waker = Waker::from(Arc::new(ComeBackThenWake {
            mutex: this.mutex.clone(),
            task: cx.waker().clone(),
        }));

        Pin::new(&mut this.future).poll(&mut Context::from_waker(&waker))
    }
}

/// A waker that comes back into a mutex from its wake, before it wakes the task it stands for.
///
/// The mutex keeps its event to itself, so the waker comes back into it through the mutex's own
/// calls: it polls a fresh `lock` once, with a waker that goes nowhere, and lets go of what that
/// gave. A guard, where the mutex was free, releases the mutex and notifies its event as it goes;
/// a wait still pending gives up the listener it took. The waker is run by a notification of the
/// mutex's event, so either happens inside that notification.
struct ComeBackThenWake {
    mutex: Arc<Mutex<usize>>,
    task: Waker,
}

impl Wake for ComeBackThenWake {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        // `drop` lets go of the poll's result, and the future it polled goes at the end of this
        // statement, so whatever the poll took is let go of before the task is woken.
        drop(pin!(self.mutex.lock()).poll(&mut Context::from_waker(Waker::noop())));
        self.task.wake_by_ref();
    }
}

/// A waker that counts its wakes.
struct CountWakes(Arc<AtomicUsize>);

impl Wake for CountWakes {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}
