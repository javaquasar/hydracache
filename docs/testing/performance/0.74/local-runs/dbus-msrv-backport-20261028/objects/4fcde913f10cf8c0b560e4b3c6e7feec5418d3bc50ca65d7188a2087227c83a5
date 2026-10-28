//! Tests of [`OnceCell`], a cell that is set once and whose value tasks can wait for.
//!
//! Most of these drive the futures of a cell by hand, polling each with a waker that goes nowhere,
//! and hold an initialiser in the middle of its work with a future that stays pending until the
//! test opens a gate, so that every test says exactly who has the value and who still waits. They
//! come first: that `get` finds a value only once one is set; that `get_or_init` runs its closure
//! once, and that an initialiser that comes while another runs waits for it and gets its value;
//! what an initialiser that fails, is given up or panics leaves behind, which is an empty cell and
//! the next initialiser in line running its own closure; what `wait` waits for, and that it goes on
//! waiting through initialisers that fail or are given up; and that `set` waits for an initialiser
//! that runs, and fails or succeeds as that initialiser does, where no other initialiser is in
//! line. Then come what a value does to the wakers of the tasks that wait for it, which are those
//! of `wait`, of the initialisers that wait their turn and of `set`, the last two let out all
//! together rather than one behind the other, and the smaller conveniences (borrowing the value,
//! taking it back, making a cell, printing it).
//!
//! The last three drive cells from many threads at once. The first has threads ask for the value of
//! one cell together and checks that one initialiser runs and every thread gets its value; the
//! second that threads waiting for a value are all released by the one that initialises it; the
//! third that a value set racing the start of a wait is not missed. All three are shrunk under
//! Miri, which runs them far more slowly.

use std::{
    any::Any,
    cell::Cell,
    future::{Future, poll_fn},
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{
        Arc, Barrier,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::Poll,
    thread,
};

use futures::executor::block_on;
use ntest::timeout;

use super::{counting_waker, handoff, linger, poll_once, poll_with, ready};
use crate::{lock::OnceCell, tests::yield_now};

/// A new cell has no value, which `get` reports without waiting; once a value is set, `get` hands
/// it out.
#[test]
fn get_finds_no_value_until_one_is_set() {
    let cell = OnceCell::new();
    assert_eq!(cell.get(), None);

    assert_eq!(ready(cell.set(1)), Ok(&1));

    assert_eq!(cell.get(), Some(&1));
}

/// `get_or_init` calls its closure on a cell with no value, and hands out what the future it makes
/// resolves to. Later calls hand out that same value, the very same reference, and call no closure
/// of their own.
#[test]
fn get_or_init_runs_its_closure_once_and_hands_back_the_same_value() {
    let cell = OnceCell::new();
    let runs = Cell::new(0);

    let first = ready(cell.get_or_init(|| {
        runs.set(runs.get() + 1);
        async { 1 }
    }));
    let second = ready(cell.get_or_init(|| {
        runs.set(runs.get() + 1);
        async { 2 }
    }));

    assert_eq!((first, second), (&1, &1));
    assert!(std::ptr::eq(first, second));
    assert_eq!(runs.get(), 1);
}

/// An initialiser that comes while another runs waits for it, and hands out the value that one sets
/// without calling its own closure. Until the first initialiser's future completes, the cell has no
/// value for `get` to find.
#[test]
fn a_concurrent_initialiser_waits_for_the_first_and_gets_its_value() {
    let cell = OnceCell::new();
    let gate = Gate::default();
    let second_runs = Cell::new(0);
    let mut first = Box::pin(cell.get_or_init(|| gate.hold(1)));
    let mut second = Box::pin(cell.get_or_init(|| {
        second_runs.set(second_runs.get() + 1);
        gate.hold(2)
    }));
    assert!(poll_once(&mut first).is_pending());
    assert!(poll_once(&mut second).is_pending());
    assert_eq!(cell.get(), None);

    gate.open();

    assert_eq!(poll_once(&mut first), Poll::Ready(&1));
    assert_eq!(poll_once(&mut second), Poll::Ready(&1));
    assert_eq!(second_runs.get(), 0);
}

/// An initialiser whose future fails hands the error to its caller alone and leaves the cell
/// empty, so the next initialiser runs its closure, and its value is the one the cell keeps.
#[test]
fn a_failing_initialiser_leaves_the_cell_empty_and_the_next_one_runs() {
    let cell = OnceCell::<u8>::new();

    let failed = ready(cell.get_or_try_init(|| async { Err("failed") }));
    assert_eq!(failed, Err("failed"));
    assert_eq!(cell.get(), None);

    let made = ready(cell.get_or_try_init(|| async { Ok::<_, &str>(1) }));
    assert_eq!(made, Ok(&1));
    assert_eq!(cell.get(), Some(&1));
}

/// An initialiser that waits behind one that fails does not take the failure for its answer: it
/// runs its own closure, once the failing one is gone, and sets the value.
#[test]
fn an_initialiser_waiting_behind_a_failing_one_runs_its_own_closure() {
    let cell = OnceCell::new();
    let gate = Gate::default();
    let mut failing = Box::pin(cell.get_or_try_init(|| gate.hold(Err("failed"))));
    let mut waiting = Box::pin(cell.get_or_init(|| gate.hold(2)));
    assert!(poll_once(&mut failing).is_pending());
    assert!(poll_once(&mut waiting).is_pending());

    gate.open();

    assert_eq!(poll_once(&mut failing), Poll::Ready(Err("failed")));
    assert_eq!(cell.get(), None);
    assert_eq!(poll_once(&mut waiting), Poll::Ready(&2));
}

/// An initialiser given up while its future is pending, by dropping the call, leaves the cell empty
/// and lets go of the initialising: the next initialiser, which was waiting, is woken, and runs its
/// own closure.
#[test]
fn a_cancelled_initialiser_lets_the_next_waiting_initialiser_run() {
    let cell = OnceCell::new();
    let (woken, waker) = counting_waker();
    let gate = Gate::default();
    let mut cancelled = Box::pin(cell.get_or_init(|| gate.hold(1)));
    let mut waiting = Box::pin(cell.get_or_init(|| async { 2 }));
    assert!(poll_once(&mut cancelled).is_pending());
    assert!(poll_with(&mut waiting, &waker).is_pending());

    drop(cancelled);

    assert_eq!(woken.load(Ordering::SeqCst), 1);
    assert_eq!(cell.get(), None);
    assert_eq!(poll_once(&mut waiting), Poll::Ready(&2));
}

/// An initialiser that panics, whether in its closure or in the future that closure makes, leaves
/// the cell empty and lets go of the initialising: nothing poisons the cell, and the next
/// initialiser runs.
#[test]
fn a_panic_in_the_initialiser_leaves_the_cell_empty_and_usable() {
    let cell = OnceCell::<u8>::new();

    let panicked = catch_unwind(AssertUnwindSafe(|| {
        ready(cell.get_or_init(|| -> std::future::Ready<u8> { panic!("the closure panicked") }))
    }));
    assert_eq!(message(panicked), "the closure panicked");
    assert_eq!(cell.get(), None);

    let panicked = catch_unwind(AssertUnwindSafe(|| {
        ready(cell.get_or_init(panics_in_the_future))
    }));
    assert_eq!(message(panicked), "the future panicked");
    assert_eq!(cell.get(), None);

    // The cell is not left held by either: this initialiser takes its turn at once.
    assert_eq!(ready(cell.get_or_init(|| async { 3 })), &3);
}

/// `wait` waits while the cell has no value and completes once another task sets one, and at once
/// for a cell that has a value already.
#[test]
fn wait_completes_once_another_task_sets_the_value() {
    let cell = OnceCell::new();
    let mut waiting = Box::pin(cell.wait());
    assert!(poll_once(&mut waiting).is_pending());

    assert_eq!(ready(cell.set(4)), Ok(&4));

    assert_eq!(poll_once(&mut waiting), Poll::Ready(&4));
    assert_eq!(ready(cell.wait()), &4);
}

/// `wait` never initialises the cell, and does not take an initialiser's failure, or its being
/// given up, for an answer: it goes on waiting, for the value of whichever task sets one.
#[test]
fn wait_keeps_waiting_through_initialisers_that_fail_or_are_cancelled() {
    let cell = OnceCell::new();
    let gate = Gate::default();
    let mut waiting = Box::pin(cell.wait());
    assert!(poll_once(&mut waiting).is_pending());

    let mut cancelled = Box::pin(cell.get_or_init(|| gate.hold(1)));
    assert!(poll_once(&mut cancelled).is_pending());
    drop(cancelled);
    assert!(poll_once(&mut waiting).is_pending());
    assert_eq!(cell.get(), None);

    let failed = ready(cell.get_or_try_init(|| async { Err("failed") }));
    assert_eq!(failed, Err("failed"));
    assert!(poll_once(&mut waiting).is_pending());

    assert_eq!(ready(cell.get_or_init(|| async { 5 })), &5);
    assert_eq!(poll_once(&mut waiting), Poll::Ready(&5));
}

/// `set` stores its value in a cell that has none, and hands it out; on a cell that has one it
/// fails, handing the value it was given back, and the value the cell has stays.
#[test]
fn set_succeeds_on_an_empty_cell_and_fails_on_a_set_one() {
    let cell = OnceCell::new();

    assert_eq!(ready(cell.set(1)), Ok(&1));
    assert_eq!(ready(cell.set(2)), Err(2));

    assert_eq!(cell.get(), Some(&1));
}

/// `set` waits for an initialiser that runs, as it would otherwise set a value the initialiser then
/// has no say over. Where the initialiser sets a value, `set` fails, handing its value back.
#[test]
fn set_waits_for_an_initialiser_in_progress_and_fails_when_it_succeeds() {
    let cell = OnceCell::new();
    let gate = Gate::default();
    let mut initialising = Box::pin(cell.get_or_init(|| gate.hold(1)));
    let mut setting = Box::pin(cell.set(9));
    assert!(poll_once(&mut initialising).is_pending());
    assert!(poll_once(&mut setting).is_pending());

    gate.open();

    assert_eq!(poll_once(&mut initialising), Poll::Ready(&1));
    assert_eq!(poll_once(&mut setting), Poll::Ready(Err(9)));
    assert_eq!(cell.get(), Some(&1));
}

/// `set` waits for an initialiser that runs, and sets its value if that initialiser fails, as the
/// cell is still empty.
#[test]
fn set_waits_for_an_initialiser_in_progress_and_succeeds_when_it_fails() {
    let cell = OnceCell::new();
    let gate = Gate::default();
    let mut initialising = Box::pin(cell.get_or_try_init(|| gate.hold(Err("failed"))));
    let mut setting = Box::pin(cell.set(9));
    assert!(poll_once(&mut initialising).is_pending());
    assert!(poll_once(&mut setting).is_pending());

    gate.open();

    assert_eq!(poll_once(&mut initialising), Poll::Ready(Err("failed")));
    assert_eq!(poll_once(&mut setting), Poll::Ready(Ok(&9)));
    assert_eq!(cell.get(), Some(&9));
}

/// A `set` given up while it waits for an initialiser leaves the cell as it was: the initialiser
/// goes on, and sets its value.
#[test]
fn a_cancelled_set_leaves_the_initialiser_alone() {
    let cell = OnceCell::new();
    let gate = Gate::default();
    let mut initialising = Box::pin(cell.get_or_init(|| gate.hold(1)));
    let mut setting = Box::pin(cell.set(9));
    assert!(poll_once(&mut initialising).is_pending());
    assert!(poll_once(&mut setting).is_pending());

    drop(setting);
    gate.open();

    assert_eq!(poll_once(&mut initialising), Poll::Ready(&1));
}

/// A value set by `set` wakes the task of every `wait` for it, once each.
#[test]
fn setting_the_value_wakes_every_task_that_waits_for_it() {
    let cell = OnceCell::new();
    let (first_woken, first_waker) = counting_waker();
    let (second_woken, second_waker) = counting_waker();
    let mut first = Box::pin(cell.wait());
    let mut second = Box::pin(cell.wait());
    assert!(poll_with(&mut first, &first_waker).is_pending());
    assert!(poll_with(&mut second, &second_waker).is_pending());
    assert_eq!(first_woken.load(Ordering::SeqCst), 0);
    assert_eq!(second_woken.load(Ordering::SeqCst), 0);

    assert_eq!(ready(cell.set(1)), Ok(&1));

    assert_eq!(first_woken.load(Ordering::SeqCst), 1);
    assert_eq!(second_woken.load(Ordering::SeqCst), 1);
    assert_eq!(poll_with(&mut first, &first_waker), Poll::Ready(&1));
    assert_eq!(poll_with(&mut second, &second_waker), Poll::Ready(&1));
}

/// A value made by an initialiser wakes the task of every `wait` for it, once each, as one set by
/// `set` does.
#[test]
fn initialising_the_cell_wakes_every_task_that_waits_for_it() {
    let cell = OnceCell::new();
    let (first_woken, first_waker) = counting_waker();
    let (second_woken, second_waker) = counting_waker();
    let mut first = Box::pin(cell.wait());
    let mut second = Box::pin(cell.wait());
    assert!(poll_with(&mut first, &first_waker).is_pending());
    assert!(poll_with(&mut second, &second_waker).is_pending());

    assert_eq!(ready(cell.get_or_init(|| async { 1 })), &1);

    assert_eq!(first_woken.load(Ordering::SeqCst), 1);
    assert_eq!(second_woken.load(Ordering::SeqCst), 1);
    assert_eq!(poll_with(&mut first, &first_waker), Poll::Ready(&1));
    assert_eq!(poll_with(&mut second, &second_waker), Poll::Ready(&1));
}

/// A value made by an initialiser wakes the task of every call that waits for its turn to
/// initialise the cell, not just the next in line: the second of two such calls is let out as soon
/// as it is polled, while the first, which was woken before it, has not run yet, and each hands out
/// the value without running its closure.
#[test]
fn initialising_the_cell_wakes_every_task_that_waits_to_initialise_it() {
    let cell = OnceCell::new();
    let gate = Gate::default();
    let runs = Cell::new(0);
    let (first_woken, first_waker) = counting_waker();
    let (second_woken, second_waker) = counting_waker();
    let mut initialising = Box::pin(cell.get_or_init(|| gate.hold(1)));
    let mut first = Box::pin(cell.get_or_init(|| {
        runs.set(runs.get() + 1);
        async { 2 }
    }));
    let mut second = Box::pin(cell.get_or_init(|| {
        runs.set(runs.get() + 1);
        async { 3 }
    }));
    assert!(poll_once(&mut initialising).is_pending());
    assert!(poll_with(&mut first, &first_waker).is_pending());
    assert!(poll_with(&mut second, &second_waker).is_pending());
    assert_eq!(first_woken.load(Ordering::SeqCst), 0);
    assert_eq!(second_woken.load(Ordering::SeqCst), 0);

    gate.open();
    assert_eq!(poll_once(&mut initialising), Poll::Ready(&1));

    assert_ne!(first_woken.load(Ordering::SeqCst), 0);
    assert_ne!(second_woken.load(Ordering::SeqCst), 0);
    // The second is let out though the first, woken before it, has not run yet.
    assert_eq!(poll_with(&mut second, &second_waker), Poll::Ready(&1));
    assert_eq!(poll_with(&mut first, &first_waker), Poll::Ready(&1));
    assert_eq!(runs.get(), 0);
}

/// A value made by an initialiser wakes the task of a `set` that waits for its turn as well, and
/// the `set` fails, handing its value back, as soon as it is polled: it does not wait for the
/// initialiser that is in line ahead of it to take its turn, which has not run yet, nor does that
/// initialiser run its closure.
#[test]
fn initialising_the_cell_wakes_a_set_that_waits_behind_an_initialiser() {
    let cell = OnceCell::new();
    let gate = Gate::default();
    let runs = Cell::new(0);
    let (setting_woken, setting_waker) = counting_waker();
    let mut initialising = Box::pin(cell.get_or_init(|| gate.hold(1)));
    let mut queued = Box::pin(cell.get_or_init(|| {
        runs.set(runs.get() + 1);
        async { 2 }
    }));
    let mut setting = Box::pin(cell.set(3));
    assert!(poll_once(&mut initialising).is_pending());
    assert!(poll_once(&mut queued).is_pending());
    assert!(poll_with(&mut setting, &setting_waker).is_pending());
    assert_eq!(setting_woken.load(Ordering::SeqCst), 0);

    gate.open();
    assert_eq!(poll_once(&mut initialising), Poll::Ready(&1));

    assert_ne!(setting_woken.load(Ordering::SeqCst), 0);
    // The `set` is let out though the initialiser ahead of it in line has not run yet.
    assert_eq!(poll_with(&mut setting, &setting_waker), Poll::Ready(Err(3)));
    assert_eq!(poll_once(&mut queued), Poll::Ready(&1));
    assert_eq!(runs.get(), 0);
}

/// A cell borrowed mutably has no initialiser to wait for, so `get_mut` and `take` reach the value
/// without a wait, and `into_inner` hands it back: each finds nothing in a cell with no value. A
/// cell whose value was taken has none, and can be set again.
#[test]
fn get_mut_take_and_into_inner_reach_the_value_without_a_wait() {
    let mut cell = OnceCell::new();
    assert_eq!(cell.get_mut(), None);
    assert_eq!(cell.take(), None);

    assert_eq!(ready(cell.set(vec![1])), Ok(&vec![1]));
    cell.get_mut().expect("the cell has a value").push(2);
    assert_eq!(cell.get(), Some(&vec![1, 2]));

    assert_eq!(cell.take(), Some(vec![1, 2]));
    assert_eq!(cell.get(), None);
    assert_eq!(ready(cell.set(vec![3])), Ok(&vec![3]));
    assert_eq!(cell.into_inner(), Some(vec![3]));
    assert_eq!(OnceCell::<u8>::new().into_inner(), None);
}

/// A cell made by `default` has no value, whatever its type, and one made by `from` has the value
/// it is given, which a `set` cannot change and a `wait` finds at once.
#[test]
fn a_cell_is_made_from_a_default_or_from_a_value() {
    /// A type with no `Default`, to show the cell needs none.
    struct NotDefault;

    assert!(OnceCell::<NotDefault>::default().get().is_none());

    let cell = OnceCell::from(7);
    assert_eq!(cell.get(), Some(&7));
    assert_eq!(ready(cell.set(8)), Err(8));
    assert_eq!(ready(cell.wait()), &7);
}

/// A cell prints as `OnceCell` around its value, or around `<uninit>` while it has none, with the
/// options of the format passed on to the value.
#[test]
fn a_cell_prints_its_value_or_a_placeholder() {
    let cell = OnceCell::new();
    assert_eq!(format!("{cell:?}"), "OnceCell(<uninit>)");

    assert_eq!(ready(cell.set(1)), Ok(&1));

    assert_eq!(format!("{cell:?}"), "OnceCell(1)");
    assert_eq!(format!("{cell:#?}"), "OnceCell(\n    1,\n)");
}

/// Threads that ask for the value of one cell together: one initialiser runs, however many of them
/// find the cell empty, and every thread is handed its value. The initialiser yields once, so that
/// the other threads find the cell empty and wait their turn. A thread that waits for an
/// initialiser to finish and is never woken would wait for good, which the timeout would catch.
#[test]
#[timeout(15000)]
fn threads_initialising_at_once_run_one_initialiser_and_share_its_value() {
    let (threads, rounds) = if cfg!(miri) { (3, 5) } else { (6, 300) };
    let cells: Arc<Vec<_>> = Arc::new((0..rounds).map(|_| OnceCell::new()).collect());
    let runs: Arc<Vec<_>> = Arc::new((0..rounds).map(|_| AtomicUsize::new(0)).collect());
    let start = Arc::new(Barrier::new(threads));

    let workers: Vec<_> = (0..threads)
        .map(|index| {
            let cells = cells.clone();
            let runs = runs.clone();
            let start = start.clone();
            thread::spawn(move || {
                (0..rounds)
                    .map(|round| {
                        // Lets the threads in together, so that they contend for each cell.
                        start.wait();
                        *block_on(cells[round].get_or_init(|| async {
                            runs[round].fetch_add(1, Ordering::SeqCst);
                            yield_now().await;
                            // The value says which thread made it.
                            index
                        }))
                    })
                    .collect::<Vec<usize>>()
            })
        })
        .collect();
    let seen: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect();

    for (round, runs) in runs.iter().enumerate() {
        assert_eq!(runs.load(Ordering::SeqCst), 1);
        // Every thread was handed the value that the one initialiser made.
        let value = seen[0][round];
        assert!(value < threads);
        assert!(seen.iter().all(|seen| seen[round] == value));
    }
}

/// Threads that wait for the value of a cell are all released by the one task that initialises it,
/// and each hands out that task's value. A thread that is not woken when the value is set would
/// wait for good, which the timeout would catch.
#[test]
#[timeout(15000)]
fn threads_waiting_for_a_value_are_all_released_by_one_initialiser() {
    let (waiters, rounds) = if cfg!(miri) { (3, 5) } else { (6, 300) };
    let cells: Arc<Vec<_>> = Arc::new((0..rounds).map(|_| OnceCell::new()).collect());
    let start = Arc::new(Barrier::new(waiters + 1));

    let waiter_threads: Vec<_> = (0..waiters)
        .map(|_| {
            let cells = cells.clone();
            let start = start.clone();
            thread::spawn(move || {
                (0..rounds)
                    .map(|round| {
                        // Lets the threads in together, so that the waiters start to wait as the
                        // value is made.
                        start.wait();
                        *block_on(cells[round].wait())
                    })
                    .collect::<Vec<usize>>()
            })
        })
        .collect();
    for round in 0..rounds {
        start.wait();
        // The waiters get a head start that is longer each round, so that the value is set across
        // the steps of their waits.
        for _ in 0..round % 64 {
            linger();
        }
        assert_eq!(
            *block_on(cells[round].get_or_init(|| async { round + 1 })),
            round + 1
        );
    }

    for waiter in waiter_threads {
        let seen = waiter.join().unwrap();
        assert!(
            seen.iter()
                .enumerate()
                .all(|(round, &value)| value == round + 1)
        );
    }
}

/// A `wait` that starts just as another task sets the value is released, wherever the setting falls
/// among its steps: before its first check, between that check and its listener, or after both.
/// Only that one value can release the waiter, so a notification it misses leaves it waiting for
/// good, which the timeout catches.
#[test]
#[timeout(15000)]
fn a_value_set_racing_a_wait_is_not_missed() {
    let rounds = if cfg!(miri) { 20 } else { 20_000 };
    handoff(
        Arc::new(Race {
            cells: (0..rounds).map(|_| OnceCell::new()).collect(),
            set: AtomicUsize::new(0),
            waited: AtomicUsize::new(0),
        }),
        rounds,
        |race, start_waiting| {
            let round = race.set.fetch_add(1, Ordering::SeqCst);
            start_waiting();
            assert_eq!(block_on(race.cells[round].set(round)), Ok(&round));
        },
        |race| {
            let round = race.waited.fetch_add(1, Ordering::SeqCst);
            assert_eq!(*block_on(race.cells[round].wait()), round);
        },
    );
}

/// The message a panic carried, which must have been a string literal.
fn message(panicked: Result<&u8, Box<dyn Any + Send>>) -> &'static str {
    let Err(payload) = panicked else {
        panic!("the call was to panic");
    };

    payload
        .downcast_ref::<&'static str>()
        .expect("the panic carried a string literal")
}

/// An initialiser's future that panics when it is first polled.
async fn panics_in_the_future() -> u8 {
    panic!("the future panicked");
}

/// A gate that holds the futures of initialisers pending, until a test opens it.
///
/// A future held by it is pending when polled, and never registers a waker: the tests poll by hand.
#[derive(Default)]
struct Gate {
    open: AtomicBool,
}

impl Gate {
    /// A future that stays pending until the gate is open, and then resolves to `value`.
    fn hold<T>(&self, value: T) -> impl Future<Output = T> {
        let mut value = Some(value);

        poll_fn(move |_| {
            if !self.open.load(Ordering::SeqCst) {
                return Poll::Pending;
            }

            Poll::Ready(
                value
                    .take()
                    .expect("the future was polled after it completed"),
            )
        })
    }

    /// Lets the futures held so far, and any held from now on, resolve.
    fn open(&self) {
        self.open.store(true, Ordering::SeqCst);
    }
}

/// The cells and the counters of `a_value_set_racing_a_wait_is_not_missed`: each round sets, and
/// waits for, a cell of its own, which each side finds by counting its rounds.
struct Race {
    cells: Vec<OnceCell<usize>>,
    /// How many rounds the setting side has begun.
    set: AtomicUsize,
    /// How many rounds the waiting side has begun.
    waited: AtomicUsize,
}
