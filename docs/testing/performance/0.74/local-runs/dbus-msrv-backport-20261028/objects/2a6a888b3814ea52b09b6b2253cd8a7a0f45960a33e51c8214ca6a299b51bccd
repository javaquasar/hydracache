//! Tests of [`Semaphore`], the lock that up to a set number of tasks may hold at once.
//!
//! Most of these drive the futures of a semaphore by hand, as the tests of the other locks do,
//! with the helpers of the module above. They come in the order of what they pin down: that the
//! semaphore admits as many holders as it has permits and makes the next one wait; that each
//! permit given back or added lets one more waiter in, two given back in a row included, before
//! either waiter woken has run; how many permits it can have free, and that it panics past that;
//! that a permit forgotten stays out; what `try_acquire` does; what the guards that hold an `Arc`
//! of the semaphore do, and that the calls handing them out wait and hold newcomers back as the
//! borrowing ones do; how a newcomer takes a free permit ahead of a waiter, how a waiter that has
//! waited for long holds newcomers back and whom it does not hold back, and what it leaves behind
//! when it is done or gives up: newcomers free to take permits again, woken only once they are,
//! and no permit lost to a waker of theirs that panics; what giving up a wait leaves behind; and
//! how the semaphore and its guards print.
//!
//! The last four drive one semaphore from many threads at once: that it never lets in more
//! holders than it has permits and deadlocks nothing on the way, that a release racing the start
//! of a wait is not missed, that a starved waiter giving up as a newcomer starts to wait does not
//! strand it, and that a thread that waits for a permit is let in however steadily others keep
//! taking them. All but the last, which ends once the waiter is let in, are shrunk under Miri,
//! which runs them far more slowly.

use std::{
    future::Future,
    panic::{AssertUnwindSafe, catch_unwind},
    pin::Pin,
    sync::{
        Arc, Barrier,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::{Context, Poll, Wake, Waker},
    thread,
};

use futures::executor::block_on;
use ntest::timeout;

use super::{handoff, linger, poll_once, ready};
use crate::lock::{PATIENCE, Semaphore, SemaphoreGuard};

/// A semaphore lets in as many holders as it has permits, each at once, and makes the next one
/// wait until a holder gives its permit back.
#[test]
fn a_semaphore_admits_up_to_its_permits_at_once() {
    let semaphore = Semaphore::new(2);
    let first = ready(semaphore.acquire());
    let _second = ready(semaphore.acquire());

    let mut third = Box::pin(semaphore.acquire());
    assert!(poll_once(&mut third).is_pending());
    drop(first);
    assert!(poll_once(&mut third).is_ready());
}

/// A permit given back lets one waiter in, the one that has waited longest, and no other: the
/// next waiter waits on until another permit comes back.
#[test]
fn a_release_lets_one_waiter_in() {
    let semaphore = Semaphore::new(1);
    let guard = ready(semaphore.acquire());
    let mut first = Box::pin(semaphore.acquire());
    assert!(poll_once(&mut first).is_pending());
    let mut second = Box::pin(semaphore.acquire());
    assert!(poll_once(&mut second).is_pending());

    drop(guard);

    let Poll::Ready(guard) = poll_once(&mut first) else {
        panic!("the release lets the first waiter in");
    };
    assert!(poll_once(&mut second).is_pending());
    drop(guard);
    assert!(poll_once(&mut second).is_ready());
}

/// Two permits given back in a row let two waiters in, though the waiter the first release woke
/// has not run yet when the second comes: each release wakes one more waiter, rather than make
/// sure one is woken, which would leave the second waiting beside a free permit.
#[test]
fn two_releases_in_a_row_let_two_waiters_in() {
    let semaphore = Semaphore::new(2);
    let first = ready(semaphore.acquire());
    let second = ready(semaphore.acquire());
    let mut waiters = [Box::pin(semaphore.acquire()), Box::pin(semaphore.acquire())];
    for waiter in &mut waiters {
        assert!(poll_once(waiter).is_pending());
    }

    drop(first);
    drop(second);

    // The first guard is kept: its release would wake the second waiter all by itself.
    let [first, second] = &mut waiters;
    let Poll::Ready(_first) = poll_once(first) else {
        panic!("the first release lets the first waiter in");
    };
    assert!(poll_once(second).is_ready());
}

/// `add_permits` lets in as many waiters as it adds permits, oldest first, and no more.
#[test]
fn add_permits_lets_that_many_waiters_in() {
    let semaphore = Semaphore::new(0);
    let mut waiters = [
        Box::pin(semaphore.acquire()),
        Box::pin(semaphore.acquire()),
        Box::pin(semaphore.acquire()),
    ];
    for waiter in &mut waiters {
        assert!(poll_once(waiter).is_pending());
    }

    semaphore.add_permits(2);

    let [first, second, third] = &mut waiters;
    let Poll::Ready(guard) = poll_once(first) else {
        panic!("the first of two permits added lets the first waiter in");
    };
    let Poll::Ready(_second) = poll_once(second) else {
        panic!("the second of two permits added lets the second waiter in");
    };
    assert!(poll_once(third).is_pending());
    drop(guard);
    assert!(poll_once(third).is_ready());
}

/// A semaphore can be made with `MAX_PERMITS` free, and `add_permits` can fill one up to that, but
/// the permits that were out when it did can still come back past it, and adding none then does
/// nothing.
#[test]
fn permits_given_back_can_take_the_count_past_max_permits() {
    let semaphore = Semaphore::new(Semaphore::MAX_PERMITS);
    let guard = semaphore
        .try_acquire()
        .expect("a new semaphore has its permits free");
    semaphore.add_permits(1);

    drop(guard);
    semaphore.add_permits(0);

    assert_eq!(
        format!("{semaphore:?}"),
        format!("Semaphore {{ permits: {} }}", Semaphore::MAX_PERMITS + 1)
    );
}

/// `add_permits` refuses to leave more than `MAX_PERMITS` free, and panics instead.
#[test]
#[should_panic(expected = "more permits than a semaphore can have free")]
fn add_permits_panics_past_max_permits() {
    let semaphore = Semaphore::new(Semaphore::MAX_PERMITS - 1);
    semaphore.add_permits(1);
    semaphore.add_permits(1);
}

/// `new` refuses to make a semaphore with more than `MAX_PERMITS` free, and panics instead.
#[test]
#[should_panic(expected = "more permits than a semaphore can have free")]
fn new_panics_past_max_permits() {
    let _semaphore = Semaphore::new(Semaphore::MAX_PERMITS + 1);
}

/// A guard forgotten keeps its permit out for good: the semaphore has one permit fewer from then
/// on, until `add_permits` adds one.
#[test]
fn forget_keeps_the_permit_out_for_good() {
    let semaphore = Semaphore::new(2);
    ready(semaphore.acquire()).forget();

    let guard = semaphore.try_acquire().expect("one permit is left");
    assert!(semaphore.try_acquire().is_none());
    drop(guard);
    let guard = semaphore.try_acquire().expect("the permit left came back");
    assert!(semaphore.try_acquire().is_none());

    semaphore.add_permits(1);
    assert!(semaphore.try_acquire().is_some());
    drop(guard);
}

/// `try_acquire` takes a free permit and fails, without waiting, while none is free, whether
/// `try_acquire` or `acquire` took them; it takes one again once a guard is gone.
#[test]
fn try_acquire_fails_with_no_permit_free_and_succeeds_once_one_is_back() {
    let semaphore = Semaphore::new(2);

    let first = semaphore
        .try_acquire()
        .expect("a new semaphore has its permits free");
    let second = ready(semaphore.acquire());
    assert!(semaphore.try_acquire().is_none());
    drop(first);
    let first = semaphore.try_acquire().expect("a permit came back");
    assert!(semaphore.try_acquire().is_none());
    drop(second);

    assert!(semaphore.try_acquire().is_some());
    drop(first);
}

/// A guard of `acquire_arc` holds the semaphore through an `Arc` of its own rather than a borrow,
/// so it outlives the borrow of the `Arc` it was taken through: it goes to another thread and
/// gives its permit back there, which lets in a task waiting on this one.
#[test]
fn an_arc_guard_goes_to_another_thread_and_gives_its_permit_back_there() {
    let semaphore = Arc::new(Semaphore::new(1));
    let guard = ready(semaphore.acquire_arc());
    let mut waiter = Box::pin(semaphore.acquire());
    assert!(poll_once(&mut waiter).is_pending());

    thread::spawn(move || drop(guard)).join().unwrap();

    assert!(poll_once(&mut waiter).is_ready());
}

/// An `Arc` guard holds a clone of the `Arc` it was taken through, whichever call made it, until it
/// is dropped or forgotten. Forgetting it lets go of that clone all the same, but keeps the permit
/// out; `try_acquire_arc` fails while no permit is free, as `try_acquire` does.
#[test]
fn an_arc_guard_holds_the_arc_until_dropped_or_forgotten() {
    let semaphore = Arc::new(Semaphore::new(1));
    let guard = ready(semaphore.acquire_arc());
    assert_eq!(Arc::strong_count(&semaphore), 2);
    assert!(semaphore.try_acquire_arc().is_none());
    drop(guard);
    assert_eq!(Arc::strong_count(&semaphore), 1);

    let guard = semaphore.try_acquire_arc().expect("the permit came back");
    assert_eq!(Arc::strong_count(&semaphore), 2);
    guard.forget();
    assert_eq!(Arc::strong_count(&semaphore), 1);
    assert!(semaphore.try_acquire_arc().is_none());

    semaphore.add_permits(1);
    assert!(semaphore.try_acquire_arc().is_some());
}

/// The calls that hand out `Arc` guards are served as the borrowing ones are: an `acquire_arc`
/// waits while no permit is free, and once it has waited for long and lost a race for a permit, it
/// holds newcomers back, `try_acquire_arc` and an `acquire_arc` that has not waited yet alike,
/// until it has had its turn.
#[test]
fn an_acquire_arc_that_waited_long_holds_newcomers_back() {
    let semaphore = Arc::new(Semaphore::new(1));
    let holder = ready(semaphore.acquire_arc());
    let mut waiter = Box::pin(semaphore.acquire_arc());
    assert!(poll_once(&mut waiter).is_pending());
    thread::sleep(PATIENCE);
    // The release wakes the waiter, which has not run yet when the first newcomer comes.
    drop(holder);
    let barging = semaphore
        .try_acquire_arc()
        .expect("a free permit is taken, whoever waits for one");

    // The waiter finds no permit free, having waited for long: it holds newcomers back from now on.
    assert!(poll_once(&mut waiter).is_pending());
    drop(barging);
    assert!(semaphore.try_acquire_arc().is_none());
    let mut newcomer = Box::pin(semaphore.acquire_arc());
    assert!(poll_once(&mut newcomer).is_pending());

    let Poll::Ready(guard) = poll_once(&mut waiter) else {
        panic!("the release of the barging guard lets the starved waiter in");
    };
    drop(guard);
    assert!(poll_once(&mut newcomer).is_ready());
}

/// A newcomer takes a free permit ahead of a waiter that has been waiting for one, but not for
/// long: the release woke that waiter, which had not run yet when the newcomer came. The waiter
/// finds no permit free and goes back to waiting, and is let in only by the newcomer's release.
#[test]
fn a_newcomer_takes_a_free_permit_ahead_of_a_waiter() {
    let semaphore = Semaphore::new(1);
    let guard = ready(semaphore.acquire());
    let mut waiter = Box::pin(semaphore.acquire());
    assert!(poll_once(&mut waiter).is_pending());
    drop(guard);

    let mut newcomer = Box::pin(semaphore.acquire());
    let Poll::Ready(newcomer) = poll_once(&mut newcomer) else {
        panic!("a free permit is taken at once, whoever waits for one");
    };

    assert!(poll_once(&mut waiter).is_pending());
    drop(newcomer);
    assert!(poll_once(&mut waiter).is_ready());
}

/// A waiter that has waited for long holds newcomers back once it has lost a race for a permit.
/// The release that wakes it is not enough: the first newcomer still takes the free permit ahead
/// of it, and it is by finding none free again that it starts to hold newcomers back. From then on
/// a free permit is not taken by `try_acquire`, nor by an `acquire` that has not waited yet, which
/// waits behind the waiter; the waiter itself takes it, and newcomers may take permits again once
/// it has.
#[test]
fn a_waiter_that_waited_long_holds_newcomers_back_from_a_free_permit() {
    let semaphore = Semaphore::new(1);
    let holder = ready(semaphore.acquire());
    let mut waiter = Box::pin(semaphore.acquire());
    assert!(poll_once(&mut waiter).is_pending());
    thread::sleep(PATIENCE);
    // The release wakes the waiter, which has not run yet when the first newcomer comes.
    drop(holder);
    let barging = ready(semaphore.acquire());

    // The waiter finds no permit free, having waited for long: it holds newcomers back from now on.
    assert!(poll_once(&mut waiter).is_pending());
    drop(barging);
    assert!(semaphore.try_acquire().is_none());
    let mut newcomer = Box::pin(semaphore.acquire());
    assert!(poll_once(&mut newcomer).is_pending());

    let Poll::Ready(guard) = poll_once(&mut waiter) else {
        panic!("the release of the barging guard lets the starved waiter in");
    };
    drop(guard);
    let Poll::Ready(guard) = poll_once(&mut newcomer) else {
        panic!("the starved waiter's release lets the newcomer in");
    };
    drop(guard);
    assert!(semaphore.try_acquire().is_some());
}

/// A waiter that holds newcomers back does not hold back those that were waiting already: one
/// that waited from before it lost its race is woken ahead of it, as the starved waiter listened
/// again behind it, and takes the free permit, which `try_acquire` still cannot. The starved
/// waiter gets a permit at the release that follows.
#[test]
fn a_waiter_waiting_already_takes_a_permit_ahead_of_a_starved_one() {
    let semaphore = Semaphore::new(1);
    let holder = ready(semaphore.acquire());
    let mut starved = Box::pin(semaphore.acquire());
    assert!(poll_once(&mut starved).is_pending());
    let mut waiting = Box::pin(semaphore.acquire());
    assert!(poll_once(&mut waiting).is_pending());
    thread::sleep(PATIENCE);
    drop(holder);
    let barging = ready(semaphore.acquire());
    assert!(poll_once(&mut starved).is_pending());

    drop(barging);
    assert!(semaphore.try_acquire().is_none());
    let Poll::Ready(guard) = poll_once(&mut waiting) else {
        panic!("a waiter that was waiting already takes the free permit");
    };
    assert!(poll_once(&mut starved).is_pending());
    drop(guard);
    assert!(poll_once(&mut starved).is_ready());
}

/// A waiter that holds newcomers back stops doing so when its `acquire` future is dropped: the
/// permit that was free, and yet not taken by a newcomer, is taken by one again, with no release
/// needed.
#[test]
fn a_starved_acquire_dropped_lets_newcomers_take_permits_again() {
    let semaphore = Semaphore::new(1);
    let waiter = starved_acquire(&semaphore);

    drop(waiter);

    assert!(semaphore.try_acquire().is_some());
}

/// A waiter that holds newcomers back, dropped after the release woke it, passes the wake-up on
/// to the newcomer it held back, which waits behind it: that release is the only one there will
/// be, so the newcomer would otherwise wait for good beside a free permit.
#[test]
fn a_starved_acquire_dropped_after_its_wake_wakes_the_newcomer_behind_it() {
    let semaphore = Semaphore::new(1);
    let holder = ready(semaphore.acquire());
    let mut waiter = Box::pin(semaphore.acquire());
    assert!(poll_once(&mut waiter).is_pending());
    thread::sleep(PATIENCE);
    drop(holder);
    let barging = ready(semaphore.acquire());
    assert!(poll_once(&mut waiter).is_pending());
    let mut newcomer = Box::pin(semaphore.acquire());
    assert!(poll_once(&mut newcomer).is_pending());

    // The release notifies the waiter, which is then dropped without ever polling that
    // notification: it has to reach the newcomer instead.
    drop(barging);
    drop(waiter);
    assert!(poll_once(&mut newcomer).is_ready());
}

/// A waiter that holds newcomers back while several permits are free wakes as many of the
/// newcomers it held back as there are permits left once it has taken its own. The releases that
/// freed those permits came while the waiter alone was listening, so they woke nobody else, and
/// the newcomers began to wait after them: waking one newcomer alone would leave the next waiting
/// beside a free permit, for want of a release that may never come.
#[test]
fn a_starved_acquire_wakes_as_many_newcomers_as_there_are_permits_free() {
    let semaphore = Semaphore::new(3);
    let first = ready(semaphore.acquire());
    let second = ready(semaphore.acquire());
    let third = ready(semaphore.acquire());
    let mut waiter = Box::pin(semaphore.acquire());
    assert!(poll_once(&mut waiter).is_pending());
    thread::sleep(PATIENCE);
    drop(first);
    let barging = ready(semaphore.acquire());
    assert!(poll_once(&mut waiter).is_pending());
    // Three permits free, and the starved waiter, notified already, is the only one listening.
    drop(second);
    drop(third);
    drop(barging);
    let mut newcomers = [
        Box::pin(semaphore.acquire()),
        Box::pin(semaphore.acquire()),
        Box::pin(semaphore.acquire()),
    ];
    for newcomer in &mut newcomers {
        assert!(poll_once(newcomer).is_pending());
    }

    let Poll::Ready(guard) = poll_once(&mut waiter) else {
        panic!("the starved waiter takes one of the free permits");
    };

    // The newcomers' guards are kept: their releases would wake the third all by themselves.
    let [first, second, third] = &mut newcomers;
    let Poll::Ready(_first) = poll_once(first) else {
        panic!("the starved waiter wakes the first newcomer for a free permit");
    };
    let Poll::Ready(_second) = poll_once(second) else {
        panic!("the starved waiter wakes the second newcomer for the last free permit");
    };
    assert!(poll_once(third).is_pending());
    drop(guard);
    assert!(poll_once(third).is_ready());
}

/// A waiter that holds newcomers back stops doing so before it wakes the newcomers it held back:
/// one that its going wakes, and that tries again from inside that wake, takes the free permit.
/// Woken first, it would find newcomers still held back, and wait beside the permit for good.
#[test]
fn a_starved_acquire_stops_holding_newcomers_back_before_it_wakes_them() {
    let semaphore = Arc::new(Semaphore::new(1));
    let waiter = starved_acquire(&semaphore);
    // Gets the notification the starved waiter passes on as its listener is dropped.
    let mut first = Box::pin(semaphore.acquire());
    assert!(poll_once(&mut first).is_pending());
    let wake = Arc::new(TakeOnWake {
        semaphore: semaphore.clone(),
        took: AtomicBool::new(false),
    });
    let waker = Waker::from(wake.clone());
    let mut second = Box::pin(semaphore.acquire());
    let polled = second.as_mut().poll(&mut Context::from_waker(&waker));
    assert!(
        polled.is_pending(),
        "the starved waiter holds newcomers back"
    );

    drop(waiter);

    assert!(
        wake.took.load(Ordering::SeqCst),
        "the newcomer woken takes the free permit"
    );
}

/// A starved waiter that takes a permit while others are free wakes a newcomer it held back for
/// them, and a waker of that newcomer's that panics leaves no permit lost: the waiter's guard is
/// made before the wake, and gives its permit back as the panic unwinds.
#[test]
fn a_panicking_waker_woken_as_a_starved_acquire_completes_loses_no_permit() {
    let semaphore = Semaphore::new(2);
    let first = ready(semaphore.acquire());
    let second = ready(semaphore.acquire());
    let mut waiter = Box::pin(semaphore.acquire());
    assert!(poll_once(&mut waiter).is_pending());
    thread::sleep(PATIENCE);
    drop(first);
    let barging = ready(semaphore.acquire());
    assert!(poll_once(&mut waiter).is_pending());
    drop(barging);
    drop(second);
    let waker = Waker::from(Arc::new(PanicOnWake));
    let mut newcomer = Box::pin(semaphore.acquire());
    let polled = newcomer.as_mut().poll(&mut Context::from_waker(&waker));
    assert!(
        polled.is_pending(),
        "the starved waiter holds the newcomer back"
    );

    assert!(catch_unwind(AssertUnwindSafe(|| poll_once(&mut waiter))).is_err());
    drop(waiter);
    drop(ready(newcomer));

    let _first = semaphore.try_acquire().expect("no permit is lost");
    let _second = semaphore.try_acquire().expect("no permit is lost");
}

/// An `acquire` future dropped after the release woke it, before it took the permit, passes the
/// wake-up on, so the waiter behind it is let in and not left waiting for a release that never
/// comes.
#[test]
fn a_dropped_acquire_future_does_not_strand_the_next_waiter() {
    let semaphore = Semaphore::new(1);
    let guard = ready(semaphore.acquire());
    let mut first = Box::pin(semaphore.acquire());
    assert!(poll_once(&mut first).is_pending());
    let mut second = Box::pin(semaphore.acquire());
    assert!(poll_once(&mut second).is_pending());
    // The release notifies `first`, which is then dropped without ever polling that
    // notification: it has to reach `second` instead.
    drop(guard);
    drop(first);
    assert!(poll_once(&mut second).is_ready());
}

/// A semaphore prints how many permits it has free, and each guard prints the semaphore it took
/// its permit from.
#[test]
fn a_semaphore_and_its_guards_print_the_permits_free() {
    let semaphore = Arc::new(Semaphore::new(2));
    assert_eq!(format!("{semaphore:?}"), "Semaphore { permits: 2 }");

    let guard = ready(semaphore.acquire());
    assert_eq!(
        format!("{guard:?}"),
        "SemaphoreGuard(Semaphore { permits: 1 })"
    );
    let arc_guard = ready(semaphore.acquire_arc());
    assert_eq!(
        format!("{arc_guard:?}"),
        "SemaphoreGuardArc(Semaphore { permits: 0 })"
    );
    drop(guard);
    drop(arc_guard);

    assert_eq!(format!("{semaphore:?}"), "Semaphore { permits: 2 }");
}

/// Threads that take permits over and over, each counting itself among the holders while it has
/// one: a holder let in beyond the permits would take the count past them, and a release that no
/// waiting thread heard of would leave a thread waiting for good, which the timeout would catch.
#[test]
#[timeout(15000)]
fn a_semaphore_admits_no_more_holders_than_its_permits_across_threads() {
    let (permits, threads, rounds) = if cfg!(miri) { (2, 3, 20) } else { (3, 8, 1000) };
    let semaphore = Arc::new(Semaphore::new(permits));
    let holders = Arc::new(AtomicUsize::new(0));
    let start = Arc::new(Barrier::new(threads));

    let takers: Vec<_> = (0..threads)
        .map(|_| {
            let semaphore = semaphore.clone();
            let holders = holders.clone();
            let start = start.clone();
            thread::spawn(move || {
                // Lets the threads in together, so that they contend from their first round rather
                // than each running its rounds before the next is spawned.
                start.wait();
                for _ in 0..rounds {
                    let guard = block_on(semaphore.acquire());
                    let now = holders.fetch_add(1, Ordering::Relaxed) + 1;
                    assert!(now <= permits, "{now} holders of {permits} permits");
                    // Gives a holder too many, were one let in, a moment to come in.
                    linger();
                    holders.fetch_sub(1, Ordering::Relaxed);
                    drop(guard);
                }
            })
        })
        .collect();
    for taker in takers {
        taker.join().unwrap();
    }

    let guards: Vec<_> = (0..permits)
        .map(|_| semaphore.try_acquire().expect("every permit came back"))
        .collect();
    assert!(semaphore.try_acquire().is_none());
    drop(guards);
}

/// An `acquire` that starts to wait just as the holder gives the permit back is let in, wherever
/// the release falls among its steps: before its first try, between that try and its listener, or
/// after both. Only that one release can let the waiter in, so a release it misses leaves it
/// waiting for good, which the timeout catches.
#[test]
#[timeout(15000)]
fn a_release_racing_an_acquire_is_not_missed() {
    let rounds = if cfg!(miri) { 20 } else { 20_000 };
    handoff(
        Arc::new(Semaphore::new(1)),
        rounds,
        |semaphore, start_waiting| {
            let _guard = semaphore
                .try_acquire()
                .expect("nothing holds the permit between rounds");
            start_waiting();
        },
        |semaphore| drop(block_on(semaphore.acquire())),
    );
}

/// An `acquire` that starts to wait just as a starved waiter's `acquire` future is dropped is let
/// in. The permit is free, and its last release went to the starved waiter, which passes it on as
/// it is dropped, before it stops holding newcomers back: a newcomer that listens in between is
/// held back from the free permit, and only the starved waiter's going can wake it. The drop is
/// spread across the steps of the newcomer's wait, so that a round may fall in between, and a
/// newcomer that nothing wakes waits for good, which the timeout catches.
#[test]
#[timeout(15000)]
fn a_starved_acquire_dropped_racing_an_acquire_does_not_strand_it() {
    let rounds = if cfg!(miri) { 20 } else { 2_000 };
    handoff(
        Arc::new(Semaphore::new(1)),
        rounds,
        |semaphore, start_waiting| {
            let starved = starved_acquire(semaphore);
            start_waiting();
            drop(starved);
        },
        |semaphore| drop(block_on(semaphore.acquire())),
    );
}

/// A thread that waits for a permit is let in, though three others keep taking the two there are:
/// each takes one again as soon as it has given it back, with a moment of holding it in between,
/// so that a permit is free for an instant at a time. The waiter gets in either by finding one
/// free in that instant or, once it has waited for long and lost a race, by holding the others
/// back until it has a permit; did it never get in, the timeout would catch it. No waiter is left
/// counted as holding newcomers back when they are done, or the permits would not be taken
/// afterwards.
#[test]
#[timeout(15000)]
fn a_semaphore_that_three_threads_keep_taking_still_lets_a_fourth_in() {
    let semaphore = Arc::new(Semaphore::new(2));
    let stop = Arc::new(AtomicBool::new(false));
    let start = Arc::new(Barrier::new(4));

    let takers: Vec<_> = (0..3)
        .map(|_| {
            let semaphore = semaphore.clone();
            let stop = stop.clone();
            let start = start.clone();
            thread::spawn(move || {
                // Lets the threads in together, so that the waiter meets the takers from the start.
                start.wait();
                while !stop.load(Ordering::Relaxed) {
                    let guard = block_on(semaphore.acquire());
                    linger();
                    drop(guard);
                }
            })
        })
        .collect();
    let waiter = thread::spawn({
        let semaphore = semaphore.clone();
        let stop = stop.clone();
        move || {
            start.wait();
            let guard = block_on(semaphore.acquire());
            // Lets the takers know that this thread has had its turn.
            stop.store(true, Ordering::Relaxed);
            drop(guard);
        }
    });
    waiter.join().unwrap();
    for taker in takers {
        taker.join().unwrap();
    }

    let first = semaphore.try_acquire().expect("every permit came back");
    assert!(semaphore.try_acquire().is_some());
    drop(first);
}

/// An `acquire` future of `semaphore`, which has one permit, that holds newcomers back, on a
/// semaphore whose permit is free and whose last release notified it: the future has waited for
/// long, and then found no permit free.
fn starved_acquire(semaphore: &Semaphore) -> Pin<Box<impl Future<Output = SemaphoreGuard<'_>>>> {
    let holder = semaphore
        .try_acquire()
        .expect("nothing holds the permit between rounds");
    let mut waiter = Box::pin(semaphore.acquire());
    assert!(poll_once(&mut waiter).is_pending());
    thread::sleep(PATIENCE);
    drop(holder);
    let barging = semaphore
        .try_acquire()
        .expect("the release left the permit free");
    assert!(poll_once(&mut waiter).is_pending());
    drop(barging);
    assert!(
        semaphore.try_acquire().is_none(),
        "the waiter holds newcomers back"
    );

    waiter
}

/// A waker that tries a fresh `acquire` of `semaphore` from its wake, and records whether it took
/// a permit.
struct TakeOnWake {
    semaphore: Arc<Semaphore>,
    took: AtomicBool,
}

impl Wake for TakeOnWake {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        // The guard the poll may complete with, and the future, are dropped with this statement.
        let took = poll_once(&mut Box::pin(self.semaphore.acquire())).is_ready();
        self.took.store(took, Ordering::SeqCst);
    }
}

/// A waker whose `wake` panics, for a wake-up that goes wrong.
struct PanicOnWake;

impl Wake for PanicOnWake {
    fn wake(self: Arc<Self>) {
        panic!("a waker that panics");
    }
}
