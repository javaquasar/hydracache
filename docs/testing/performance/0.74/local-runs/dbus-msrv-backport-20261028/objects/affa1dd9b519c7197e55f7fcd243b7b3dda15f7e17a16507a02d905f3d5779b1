//! Tests of [`Barrier`], which makes tasks wait for each other.
//!
//! Most of these drive the `wait` futures of a barrier by hand, polling each with a waker that goes
//! nowhere, so that every test says exactly who a trip releases and who still waits. They come
//! first: that the tasks of a round wait until the last has arrived and are then released together,
//! with the task whose arrival completes the round as its leader and nobody else; that the barrier
//! serves round after round, and that a task released by one round is not held by the next; what a
//! barrier made for no tasks or for one does; and that a `wait` future arrives when it is first
//! polled, not when it is made. Then come what giving up a wait leaves behind, which is the
//! arrival taken back unless its round was completed already, what a trip does to the wakers of the
//! tasks that wait, and how a barrier and its results print.
//!
//! The last two drive one barrier from threads. The first has several threads meet over many
//! rounds, and checks that each round has exactly one leader and that no thread leaves a round
//! before the last has arrived, nor gets more than a round ahead of the others; the second that a
//! trip racing the start of a wait is not missed. Both are shrunk under Miri, which runs them far
//! more slowly.

use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::Poll,
    thread,
};

use futures::executor::block_on;
use ntest::timeout;

use super::{counting_waker, handoff, poll_once, poll_with, ready};
use crate::lock::Barrier;

/// The tasks of a round wait until the last of them has arrived, and are all released by that
/// arrival. A task that is polled again while it waits does not count a second time.
#[test]
fn waiters_wait_until_the_last_task_arrives() {
    let barrier = Barrier::new(3);
    let mut first = Box::pin(barrier.wait());
    let mut second = Box::pin(barrier.wait());
    assert!(poll_once(&mut first).is_pending());
    assert!(poll_once(&mut second).is_pending());
    // Two tasks have arrived, however often they are polled: the third is still to come.
    assert!(poll_once(&mut first).is_pending());
    assert!(poll_once(&mut second).is_pending());

    let mut third = Box::pin(barrier.wait());
    assert!(poll_once(&mut third).is_ready());
    assert!(poll_once(&mut first).is_ready());
    assert!(poll_once(&mut second).is_ready());
}

/// The task whose arrival completes the round is its leader, whichever task that is, and no other
/// task of the round is.
#[test]
fn the_task_that_completes_the_round_is_its_leader() {
    let barrier = Barrier::new(3);
    let mut first = Box::pin(barrier.wait());
    let mut second = Box::pin(barrier.wait());
    let mut third = Box::pin(barrier.wait());
    // The tasks arrive in another order than they were made in, so that the leader is not the one
    // made last.
    assert!(poll_once(&mut third).is_pending());
    assert!(poll_once(&mut first).is_pending());
    let Poll::Ready(leader) = poll_once(&mut second) else {
        panic!("the last task to arrive completes the round");
    };
    let Poll::Ready(third) = poll_once(&mut third) else {
        panic!("the arrival that completes the round releases the tasks waiting");
    };
    let Poll::Ready(first) = poll_once(&mut first) else {
        panic!("the arrival that completes the round releases the tasks waiting");
    };

    assert!(leader.is_leader());
    assert!(!third.is_leader());
    assert!(!first.is_leader());
}

/// A barrier serves round after round: once a round is complete, the next `n` arrivals complete
/// the next, each with a leader of its own.
#[test]
fn a_barrier_is_reusable_for_further_rounds() {
    let barrier = Barrier::new(2);

    for _ in 0..3 {
        let mut first = Box::pin(barrier.wait());
        let mut second = Box::pin(barrier.wait());
        assert!(poll_once(&mut first).is_pending());
        let Poll::Ready(leader) = poll_once(&mut second) else {
            panic!("the second arrival completes a round of two");
        };
        let Poll::Ready(follower) = poll_once(&mut first) else {
            panic!("the second arrival releases the first");
        };

        assert!(leader.is_leader());
        assert!(!follower.is_leader());
    }
}

/// A task released by a round is released whatever has happened at the barrier since: where tasks
/// of the next round have arrived already by the time it is polled, it still completes, as a
/// follower, and does not count towards the round they are in.
#[test]
fn a_task_released_by_a_round_is_not_held_by_the_next() {
    let barrier = Barrier::new(2);
    let mut slow = Box::pin(barrier.wait());
    assert!(poll_once(&mut slow).is_pending());
    assert!(ready(barrier.wait()).is_leader());

    // The next round has begun before `slow` has noticed that its own is over.
    let mut early = Box::pin(barrier.wait());
    assert!(poll_once(&mut early).is_pending());

    let Poll::Ready(slow) = poll_once(&mut slow) else {
        panic!("a task whose round is complete is released");
    };
    assert!(!slow.is_leader());
    // The next round is still one arrival short, and `slow` is not the one it needs.
    assert!(poll_once(&mut early).is_pending());
    let Poll::Ready(leader) = poll_once(&mut Box::pin(barrier.wait())) else {
        panic!("the arrival of the second task completes the next round");
    };
    assert!(leader.is_leader());
    assert!(poll_once(&mut early).is_ready());
}

/// A barrier for no tasks behaves as one for a single task, as `std::sync::Barrier` does: every
/// `wait` completes at once, as the leader of a round of its own, however often it is called.
#[test]
fn a_barrier_for_zero_or_one_task_completes_every_wait_at_once() {
    for n in [0, 1] {
        let barrier = Barrier::new(n);

        for _ in 0..3 {
            assert!(ready(barrier.wait()).is_leader());
        }
    }
}

/// A `wait` future counts when it is first polled and not when it is made: of two tasks that a
/// barrier is for, one that has only made its future has not arrived, so the other waits.
#[test]
fn a_wait_that_was_never_polled_does_not_count() {
    let barrier = Barrier::new(2);
    let mut unpolled = Box::pin(barrier.wait());
    let mut waiting = Box::pin(barrier.wait());

    assert!(poll_once(&mut waiting).is_pending());
    let Poll::Ready(leader) = poll_once(&mut unpolled) else {
        panic!("the arrival of the second task completes the round");
    };
    assert!(leader.is_leader());
    assert!(poll_once(&mut waiting).is_ready());
}

/// A `wait` dropped before its round is complete takes its arrival back. The barrier needs `n`
/// arrivals again: those of the others, and one more for the task that left, so nobody is released
/// early, and nobody is left waiting for a task that is not coming either.
#[test]
fn a_wait_dropped_before_the_trip_takes_its_arrival_back() {
    let barrier = Barrier::new(3);
    let mut leaving = Box::pin(barrier.wait());
    let mut first = Box::pin(barrier.wait());
    assert!(poll_once(&mut leaving).is_pending());
    assert!(poll_once(&mut first).is_pending());

    drop(leaving);

    // Had the arrival of the task that left stayed counted, this one would complete the round.
    let mut second = Box::pin(barrier.wait());
    assert!(poll_once(&mut second).is_pending());
    assert!(poll_once(&mut first).is_pending());
    let Poll::Ready(leader) = poll_once(&mut Box::pin(barrier.wait())) else {
        panic!("the third arrival completes the round");
    };
    assert!(leader.is_leader());
    assert!(poll_once(&mut first).is_ready());
    assert!(poll_once(&mut second).is_ready());
}

/// A `wait` whose round was completed before it was dropped changes nothing: its arrival was
/// counted into that round, and does not count towards the next, which still needs all `n` of its
/// own arrivals.
#[test]
fn a_wait_dropped_after_its_round_is_complete_does_not_count_towards_the_next() {
    let barrier = Barrier::new(2);
    let mut released = Box::pin(barrier.wait());
    assert!(poll_once(&mut released).is_pending());
    assert!(ready(barrier.wait()).is_leader());

    drop(released);

    // The drop took nothing back from the next round: this task is the only one that has arrived
    // in it, and a second is needed.
    let mut first = Box::pin(barrier.wait());
    assert!(poll_once(&mut first).is_pending());
    assert!(ready(barrier.wait()).is_leader());
    assert!(poll_once(&mut first).is_ready());
}

/// Dropping every `wait` of a round leaves the barrier as it was before any task arrived: the next
/// round needs `n` arrivals of its own.
#[test]
fn a_barrier_whose_waiters_all_gave_up_starts_again() {
    let barrier = Barrier::new(2);
    let mut first = Box::pin(barrier.wait());
    assert!(poll_once(&mut first).is_pending());
    drop(first);

    let mut second = Box::pin(barrier.wait());
    assert!(poll_once(&mut second).is_pending());
    assert!(ready(barrier.wait()).is_leader());
    assert!(poll_once(&mut second).is_ready());
}

/// Completing a round wakes the task of every `wait` that waits at the barrier, once each, and
/// not the task of a `wait` that was dropped, nor anyone before then.
#[test]
fn a_trip_wakes_every_task_that_waits() {
    let barrier = Barrier::new(3);
    let (first_woken, first_waker) = counting_waker();
    let (second_woken, second_waker) = counting_waker();
    let (dropped_woken, dropped_waker) = counting_waker();
    let mut first = Box::pin(barrier.wait());
    let mut dropped = Box::pin(barrier.wait());
    let mut second = Box::pin(barrier.wait());
    assert!(poll_with(&mut first, &first_waker).is_pending());
    assert!(poll_with(&mut dropped, &dropped_waker).is_pending());
    drop(dropped);
    assert!(poll_with(&mut second, &second_waker).is_pending());
    // Two tasks wait, one short of a round: nobody is woken.
    assert_eq!(first_woken.load(Ordering::SeqCst), 0);
    assert_eq!(second_woken.load(Ordering::SeqCst), 0);

    assert!(ready(barrier.wait()).is_leader());

    assert_eq!(first_woken.load(Ordering::SeqCst), 1);
    assert_eq!(second_woken.load(Ordering::SeqCst), 1);
    assert_eq!(dropped_woken.load(Ordering::SeqCst), 0);
    assert!(poll_with(&mut first, &first_waker).is_ready());
    assert!(poll_with(&mut second, &second_waker).is_ready());
}

/// A barrier prints how many tasks make a round and how many of them wait, and a result prints
/// whether it led.
#[test]
fn a_barrier_and_its_result_print() {
    let barrier = Barrier::new(2);
    assert_eq!(format!("{barrier:?}"), "Barrier { n: 2, waiting: 0 }");

    let mut waiting = Box::pin(barrier.wait());
    assert!(poll_once(&mut waiting).is_pending());
    assert_eq!(format!("{barrier:?}"), "Barrier { n: 2, waiting: 1 }");

    let leader = ready(barrier.wait());
    assert_eq!(format!("{barrier:?}"), "Barrier { n: 2, waiting: 0 }");
    assert_eq!(
        format!("{leader:?}"),
        "BarrierWaitResult { is_leader: true }"
    );
    let Poll::Ready(follower) = poll_once(&mut waiting) else {
        panic!("the arrival that completes the round releases the tasks waiting");
    };
    assert_eq!(
        format!("{follower:?}"),
        "BarrierWaitResult { is_leader: false }"
    );
}

/// Threads that meet at a barrier over and over: every round has exactly one leader, and no thread
/// leaves a round before every thread has arrived in it, nor gets more than a round ahead of the
/// rest. A trip that no waiting thread heard of would leave a thread waiting for good, which the
/// timeout would catch.
#[test]
#[timeout(15000)]
fn every_round_has_one_leader_and_nobody_leaves_before_the_last_arrives() {
    let (threads, rounds) = if cfg!(miri) { (3, 20) } else { (6, 2000) };
    let barrier = Arc::new(Barrier::new(threads));
    // How many threads have arrived in all, counted before each arrives.
    let arrivals = Arc::new(AtomicUsize::new(0));
    // How many leaders each round has had.
    let leaders: Arc<Vec<_>> = Arc::new((0..rounds).map(|_| AtomicUsize::new(0)).collect());

    let workers: Vec<_> = (0..threads)
        .map(|_| {
            let barrier = barrier.clone();
            let arrivals = arrivals.clone();
            let leaders = leaders.clone();
            thread::spawn(move || {
                for round in 0..rounds {
                    arrivals.fetch_add(1, Ordering::SeqCst);
                    if block_on(barrier.wait()).is_leader() {
                        leaders[round].fetch_add(1, Ordering::SeqCst);
                    }

                    let arrived = arrivals.load(Ordering::SeqCst);
                    // Every thread arrived in this round before any of them left it...
                    assert!(arrived >= (round + 1) * threads);
                    // ...and none can have left the next, which this thread has yet to arrive in.
                    assert!(arrived < (round + 2) * threads);
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }

    for round_leaders in leaders.iter() {
        assert_eq!(round_leaders.load(Ordering::SeqCst), 1);
    }
}

/// A `wait` that starts just as the other task arrives is released, wherever the trip falls among
/// its steps: before its first check, between that check and its listener, or after both. Only
/// that one arrival can release the waiter, so a trip it misses leaves it waiting for good, which
/// the timeout catches.
#[test]
#[timeout(15000)]
fn a_trip_racing_a_wait_is_not_missed() {
    let rounds = if cfg!(miri) { 20 } else { 20_000 };
    handoff(
        Arc::new(Barrier::new(2)),
        rounds,
        |barrier, start_waiting| {
            start_waiting();
            block_on(barrier.wait());
        },
        |barrier| {
            block_on(barrier.wait());
        },
    );
}
