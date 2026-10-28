//! Tests of the budget of I/O operations that a poll may complete, at the level of the functions
//! that keep it: when an operation proceeds and when it yields, that a spent budget turns away
//! every operation that asks, what a poll starts with and what it leaves behind, and that a future
//! run by `unconstrained` has no budget, and leaves the poll around it with the budget it had.
//!
//! What a runtime and its I/O do with the budget is tested where they are: in the `async_io` module
//! for the handle of a source, in the `helper` module for the `block_on` of a runtime with a seat,
//! and in the `net::unix` module for a `send_to`, which counts for itself.

use std::{
    future::poll_fn,
    panic::{AssertUnwindSafe, catch_unwind},
    pin::pin,
    task::{Context, Poll},
};

use ntest::timeout;

use super::core::counting_waker;
use crate::coop::{self, budgeted, poll_proceed, spend, unconstrained};

/// A thread that is inside no poll of a runtime has no budget, so that whatever polls a future
/// there is not held to one: every operation proceeds, however many there are, and none wakes a
/// task.
#[test]
#[timeout(15000)]
fn without_a_poll_every_operation_proceeds() {
    let (woken, waker) = counting_waker();
    let mut cx = Context::from_waker(&waker);

    assert_eq!(proceeded(&mut cx, 3 * BUDGET), 3 * BUDGET);
    assert_eq!(woken.count(), 0);
}

/// An operation proceeds as long as the poll has budget, and the first one after that yields: it
/// wakes the task and does not proceed, and neither does any other until the poll is over.
#[test]
#[timeout(15000)]
fn an_operation_yields_once_the_budget_is_spent() {
    let (woken, waker) = counting_waker();
    let mut cx = Context::from_waker(&waker);

    budgeted(|| {
        assert_eq!(proceeded(&mut cx, 3 * BUDGET), BUDGET);
        assert_eq!(woken.count(), 1);

        // An operation that yielded has not completed, so it spends nothing and the next one
        // yields as well; and a spend that comes with no budget left to take it from is a
        // spend of nothing, not one that wraps around into a full budget.
        spend();
        assert!(poll_proceed(&mut cx).is_pending());
        assert_eq!(woken.count(), 2);
    });
}

/// Each poll starts with a full budget, whatever the one before it left.
#[test]
#[timeout(15000)]
fn every_poll_starts_with_a_full_budget() {
    let (woken, waker) = counting_waker();
    let mut cx = Context::from_waker(&waker);

    for _ in 0..3 {
        budgeted(|| assert_eq!(proceeded(&mut cx, 3 * BUDGET), BUDGET));
    }

    assert_eq!(woken.count(), 3);
}

/// A poll made from inside another that is budgeted too, such as the free `block_on` called from
/// inside the future of another call of it, has a budget of its own and gives the other its budget
/// back as it ends, as it was and not full again.
#[test]
#[timeout(15000)]
fn a_poll_inside_another_gives_the_budget_back() {
    let (_woken, waker) = counting_waker();
    let mut cx = Context::from_waker(&waker);

    budgeted(|| {
        assert_eq!(proceeded(&mut cx, 5), 5);

        budgeted(|| assert_eq!(proceeded(&mut cx, 3 * BUDGET), BUDGET));

        assert_eq!(proceeded(&mut cx, 3 * BUDGET), BUDGET - 5);
    });
}

/// A poll that is not budgeted, made from inside one that is, has no budget of its own: it shares
/// what is left of the budget of the poll it is inside of. Once that is spent, every operation is
/// turned away, however often it asks, and wakes its task each time.
///
/// This is what an executor of another crate meets when a task runs it. It never asks for a budget
/// of its own, and one that blocks until its future is done polls the future again as soon as an
/// operation that yielded has woken it, inside the same poll of the task, and is turned away again.
/// The calls here are as many as a few budgets, so that a test that is wrong ends rather than
/// spins.
#[test]
#[timeout(15000)]
fn a_spent_budget_turns_every_operation_away() {
    let (woken, waker) = counting_waker();
    let mut cx = Context::from_waker(&waker);
    let (nested_woken, nested_waker) = counting_waker();
    let mut nested_cx = Context::from_waker(&nested_waker);

    budgeted(|| {
        assert_eq!(proceeded(&mut cx, 5), 5);

        assert_eq!(proceeded(&mut nested_cx, 3 * BUDGET), BUDGET - 5);
        assert_eq!(nested_woken.count(), 1);

        // Asked again at once, over and over, as a blocking executor asks once it is woken.
        for asked in 2..=3 * BUDGET {
            assert!(poll_proceed(&mut nested_cx).is_pending());
            assert_eq!(nested_woken.count(), asked);
        }
        assert!(poll_proceed(&mut cx).is_pending());
    });
    assert_eq!(woken.count(), 1);
    assert_eq!(nested_woken.count(), 3 * BUDGET);
}

/// The budget goes with the outermost poll: a thread that has been through one is as free of a
/// budget as one that has not.
#[test]
#[timeout(15000)]
fn the_budget_is_gone_once_the_outermost_poll_ends() {
    let (woken, waker) = counting_waker();
    let mut cx = Context::from_waker(&waker);

    budgeted(|| assert_eq!(proceeded(&mut cx, 5), 5));

    assert_eq!(proceeded(&mut cx, 3 * BUDGET), 3 * BUDGET);
    assert_eq!(woken.count(), 0);
}

/// A panic out of a poll gives the budget back as a return does, to a thread that had none and to
/// a poll the panicking one was made from.
#[test]
#[timeout(15000)]
fn a_panic_out_of_a_poll_gives_the_budget_back() {
    let (woken, waker) = counting_waker();
    let mut cx = Context::from_waker(&waker);
    let panicking = |cx: &mut Context<'_>| {
        let panicked = catch_unwind(AssertUnwindSafe(|| {
            budgeted(|| {
                proceeded(cx, 3 * BUDGET);
                panic!("the poll panicked on purpose");
            })
        }));
        assert!(panicked.is_err());
    };

    panicking(&mut cx);
    assert_eq!(proceeded(&mut cx, 3 * BUDGET), 3 * BUDGET);

    budgeted(|| {
        assert_eq!(proceeded(&mut cx, 5), 5);
        panicking(&mut cx);
        assert_eq!(proceeded(&mut cx, 3 * BUDGET), BUDGET - 5);
    });
    // Only the polls that ran out of budget woke anything: the one that yielded after the
    // panicking poll's budget, and the one after the rest of the outer budget.
    assert_eq!(woken.count(), 3);
}

/// A future that `unconstrained` runs has no budget: every operation in its poll proceeds, however
/// many there are and however spent the budget of the poll around it is, and none wakes a task.
#[test]
#[timeout(15000)]
fn unconstrained_lets_every_operation_through() {
    let (woken, waker) = counting_waker();
    let mut cx = Context::from_waker(&waker);

    budgeted(|| {
        assert_eq!(proceeded(&mut cx, BUDGET), BUDGET);

        assert_eq!(
            polled_unconstrained(&mut cx, |cx| proceeded(cx, 3 * BUDGET)),
            3 * BUDGET
        );
    });

    assert_eq!(woken.count(), 0);
}

/// Once a poll of a future that `unconstrained` runs is over, the poll around it has its budget
/// back as it was: neither spent by the operations of that future nor full again.
#[test]
#[timeout(15000)]
fn unconstrained_gives_the_poll_around_it_its_budget_back() {
    let (woken, waker) = counting_waker();
    let mut cx = Context::from_waker(&waker);

    budgeted(|| {
        assert_eq!(proceeded(&mut cx, 5), 5);

        assert_eq!(
            polled_unconstrained(&mut cx, |cx| proceeded(cx, 3 * BUDGET)),
            3 * BUDGET
        );

        assert_eq!(proceeded(&mut cx, 3 * BUDGET), BUDGET - 5);
    });

    assert_eq!(woken.count(), 1);
}

/// A panic out of a poll of a future that `unconstrained` runs gives the poll around it its budget
/// back as a return does.
#[test]
#[timeout(15000)]
fn a_panic_out_of_unconstrained_gives_the_poll_around_it_its_budget_back() {
    let (woken, waker) = counting_waker();
    let mut cx = Context::from_waker(&waker);

    budgeted(|| {
        assert_eq!(proceeded(&mut cx, 5), 5);

        let panicked = catch_unwind(AssertUnwindSafe(|| {
            polled_unconstrained(&mut cx, |cx| {
                proceeded(cx, 3 * BUDGET);
                panic!("the future panicked on purpose");
            })
        }));
        assert!(panicked.is_err());

        assert_eq!(proceeded(&mut cx, 3 * BUDGET), BUDGET - 5);
    });

    assert_eq!(woken.count(), 1);
}

/// Outside a poll of a runtime, a future that `unconstrained` runs is as free of a budget as any
/// other, and the thread has none once its poll is over.
#[test]
#[timeout(15000)]
fn unconstrained_changes_nothing_outside_a_poll() {
    let (woken, waker) = counting_waker();
    let mut cx = Context::from_waker(&waker);

    assert_eq!(
        polled_unconstrained(&mut cx, |cx| proceeded(cx, 3 * BUDGET)),
        3 * BUDGET
    );

    assert_eq!(proceeded(&mut cx, 3 * BUDGET), 3 * BUDGET);
    assert_eq!(woken.count(), 0);
}

/// How many operations proceed, up to `most`, before one yields. Each that proceeds completes.
fn proceeded(cx: &mut Context<'_>, most: usize) -> usize {
    let mut proceeded = 0;
    while proceeded < most && poll_proceed(cx).is_ready() {
        spend();
        proceeded += 1;
    }

    proceeded
}

/// Calls `poll` in a poll, made with `cx`, of a future that [`unconstrained`] runs, and hands back
/// what it returns.
fn polled_unconstrained<R>(cx: &mut Context<'_>, poll: impl FnOnce(&mut Context<'_>) -> R) -> R {
    let mut poll = Some(poll);
    let mut future = pin!(unconstrained(poll_fn(move |cx| {
        Poll::Ready(poll.take().expect("polled once")(cx))
    })));

    match future.as_mut().poll(cx) {
        Poll::Ready(output) => output,
        Poll::Pending => unreachable!("a future that is ready at once yields to nothing"),
    }
}

/// [`coop::BUDGET`] as a count.
const BUDGET: usize = coop::BUDGET as usize;
