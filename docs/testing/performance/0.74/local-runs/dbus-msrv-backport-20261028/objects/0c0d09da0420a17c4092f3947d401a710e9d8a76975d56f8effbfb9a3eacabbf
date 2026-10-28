//! Tests of a runtime driven by nothing but the `block_on` calls made on it: the tasks, timers
//! and registrations it runs on the thread inside such a call, the wakes that reach that thread
//! from others, and the rules on who may drive it.
//!
//! Most of these run in both flavours, through [`in_both_modes`]; the ones that only mean
//! something in one flavour — a local task holding an `Rc`, a second thread asking to drive a
//! shared runtime — are written for that flavour alone.

use std::{
    any::Any,
    cell::{Cell, RefCell},
    future::{Future, Pending, pending, poll_fn},
    io::{self, Read, Write},
    mem::MaybeUninit,
    panic::{AssertUnwindSafe, catch_unwind},
    pin::{Pin, pin},
    rc::Rc,
    sync::{
        Arc, Mutex, PoisonError,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc,
    },
    task::{Context, Poll, Wake, Waker},
    thread,
    time::{Duration, Instant},
};

use futures::{Stream, StreamExt, executor::block_on, future::poll_immediate};
use ntest::timeout;
use socket2::{SockRef, Socket};

use super::{or, yield_now};
#[cfg(feature = "event")]
use crate::Event;
use crate::{
    Interest, Interval, Local, LocalRuntime, MissedTickBehavior, Mode, Registration, Runtime,
    Shared, SharedRuntime, Sleep, Task, TimedOut, Timeout,
};

/// Writes the test that follows once per flavour: a module named after it, holding a `local`
/// and a `shared` test that run its body with the mode parameter set to [`Local`] and to
/// [`Shared`].
///
/// The body reaches spawning and registering through [`TestMode`], whose bounds both flavours
/// meet, and everything else through [`Runtime`] itself.
macro_rules! in_both_modes {
    ($(#[$attr:meta])* fn $name:ident<$mode:ident>() $body:block) => {
        $(#[$attr])*
        mod $name {
            use super::*;

            #[test]
            #[timeout(15000)]
            fn local() {
                run::<Local>();
            }

            #[test]
            #[timeout(15000)]
            fn shared() {
                run::<Shared>();
            }

            fn run<$mode>()
            where
                $mode: TestMode,
            $body
        }
    };
}

in_both_modes! {
    fn a_spawned_task_hands_its_output_back<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let task = M::spawn(&runtime, "an answer", async { 42 });

        assert_eq!(runtime.block_on(task).unwrap(), 42);
    }
}

in_both_modes! {
    /// A task runs on the thread inside `block_on`, whether it was spawned from inside the call
    /// or before it.
    fn tasks_run_on_the_thread_inside_block_on<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let before = M::spawn(&runtime, "a task spawned before the call", async {
            thread::current().id()
        });

        let (before, inside) = runtime.block_on(async {
            let inside = M::spawn(&runtime, "a task spawned inside the call", async {
                thread::current().id()
            });

            (before.await.unwrap(), inside.await.unwrap())
        });

        assert_eq!(before, thread::current().id());
        assert_eq!(inside, thread::current().id());
    }
}

in_both_modes! {
    /// Work handed to a runtime nobody is driving waits for the next `block_on` on it.
    fn work_waits_for_the_next_block_on<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let ran = Arc::new(AtomicBool::new(false));
        let task = {
            let ran = ran.clone();
            M::spawn(&runtime, "a task that sets a flag", async move {
                ran.store(true, Ordering::Release);
            })
        };

        // Long enough for a thread of the runtime's own, were there one, to have run the task.
        thread::sleep(Duration::from_millis(50));
        assert!(!ran.load(Ordering::Acquire));

        runtime.block_on(task).unwrap();
        assert!(ran.load(Ordering::Acquire));
    }
}

in_both_modes! {
    #[cfg(feature = "event")]
    fn a_detached_task_runs_to_completion<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let done = Arc::new(Event::new());
        // Taken before the task is spawned, so that the announcement cannot be missed.
        let announced = done.listen();

        M::spawn(&runtime, "a detached task", async move {
            // Left waiting once, so that it is the detached task's later poll that ends it.
            yield_now().await;
            done.notify(1);
        })
        .detach();

        runtime.block_on(announced);
    }
}

in_both_modes! {
    fn dropping_a_task_cancels_and_drops_its_future<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let dropped = Arc::new(AtomicBool::new(false));
        let task = {
            let marker = SetOnDrop(dropped.clone());
            M::spawn(&runtime, "a task that never finishes", async move {
                let _marker = marker;
                pending::<()>().await;
            })
        };
        // A yield is one round of the loop, in which the task is polled and left waiting.
        runtime.block_on(yield_now());
        assert!(!dropped.load(Ordering::Acquire));

        drop(task);

        assert!(dropped.load(Ordering::Acquire));
    }
}

in_both_modes! {
    fn a_panicking_task_fails_its_handle_and_the_runtime_carries_on<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let panicking = M::spawn(&runtime, "a task that panics", async {
            panic!("the task panicked on purpose");
        });

        let error = runtime.block_on(panicking).unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::Other);
        let next = M::spawn(&runtime, "the task after it", async { 7 });
        assert_eq!(runtime.block_on(next).unwrap(), 7);
    }
}

in_both_modes! {
    /// A future whose destructor panics takes nothing with it: not a value it had already
    /// produced, not the runtime that drops it, and not the handle that cancels it, which sees
    /// the panic itself.
    fn a_panicking_destructor_is_contained<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        // A future of its own rather than an async block holding the marker: the block would drop
        // the marker inside its last poll, which is a panic of that poll.
        let going = M::spawn(&runtime, "a task that panics as it goes", ReadyThenPanicOnDrop(7));

        assert_eq!(runtime.block_on(going).unwrap(), 7);

        let cancelled = M::spawn(&runtime, "a task that never finishes", async {
            let _marker = PanicOnDrop;
            pending::<()>().await;
        });
        runtime.block_on(yield_now());
        // Idle when it is dropped, so the drop of its future is this thread's to make, and the
        // panic is the caller's to see.
        assert!(catch_unwind(AssertUnwindSafe(|| drop(cancelled))).is_err());

        let next = M::spawn(&runtime, "the task after them", async { 7 });
        assert_eq!(runtime.block_on(next).unwrap(), 7);
    }
}

in_both_modes! {
    /// A task is finished once its future has completed, whether or not anyone has awaited it
    /// yet, and stays so once its output is taken; one that is left waiting is not.
    fn a_task_is_finished_once_its_future_has_completed<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let mut completing = M::spawn(&runtime, "a task that finishes at once", async { 42 });
        let waiting = M::spawn(&runtime, "a task that never finishes", pending::<()>());
        // Neither has had its first poll.
        assert!(!completing.is_finished());
        assert!(!waiting.is_finished());

        // A yield is one round of the loop, in which each task is polled once.
        runtime.block_on(yield_now());

        assert!(completing.is_finished());
        assert!(!waiting.is_finished());
        assert_eq!(runtime.block_on(&mut completing).unwrap(), 42);
        assert!(completing.is_finished());
    }
}

in_both_modes! {
    /// A task that panics is finished, as one that completes is, and its handle reports the
    /// failure all the same.
    fn a_panicking_task_is_finished<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let panicking = M::spawn(&runtime, "a task that panics", async {
            panic!("the task panicked on purpose");
        });
        assert!(!panicking.is_finished());

        runtime.block_on(yield_now());

        assert!(panicking.is_finished());
        assert!(runtime.block_on(panicking).is_err());
    }
}

in_both_modes! {
    /// A task whose runtime goes before it ends is finished, whether it was polled before the
    /// runtime went or never was.
    fn a_task_is_finished_once_its_runtime_is_gone<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let polled = M::spawn(&runtime, "a task that never finishes", pending::<()>());
        runtime.block_on(yield_now());
        let never_polled = M::spawn(&runtime, "a task that is never polled", pending::<()>());
        assert!(!polled.is_finished());
        assert!(!never_polled.is_finished());

        drop(runtime);

        assert!(polled.is_finished());
        assert!(never_polled.is_finished());
    }
}

/// A task is not finished while its future is still being dropped, whether it ran to its end or
/// went with its runtime, polled before that or not: what the future held is gone by the time
/// anyone is told that the task is over.
#[test]
#[timeout(15000)]
fn a_local_task_is_finished_only_once_its_future_is_dropped() {
    for end in End::ALL {
        let noted =
            finished_as_future_drops(LocalRuntime::new().unwrap(), end, |runtime, future| {
                runtime.spawn("a task that notes whether it is finished", future)
            });

        assert_eq!(noted, Some(false), "{end:?}");
    }
}

/// A task is not finished while its future is still being dropped, whether it ran to its end or
/// went with its runtime, polled before that or not: what the future held is gone by the time
/// anyone is told that the task is over.
#[test]
#[timeout(15000)]
fn a_shared_task_is_finished_only_once_its_future_is_dropped() {
    for end in End::ALL {
        let noted =
            finished_as_future_drops(SharedRuntime::new().unwrap(), end, |runtime, future| {
                runtime.spawn("a task that notes whether it is finished", future)
            });

        assert_eq!(noted, Some(false), "{end:?}");
    }
}

in_both_modes! {
    /// A task cancelled before its first poll is never polled, and its future is gone by the time
    /// `cancel` returns, which leaves the wait nothing to wait for.
    fn cancelling_a_task_never_polled_drops_its_future_in_the_call<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let polled = Arc::new(AtomicBool::new(false));
        let dropped = Arc::new(AtomicBool::new(false));
        let task = {
            let polled = polled.clone();
            let marker = SetOnDrop(dropped.clone());
            M::spawn(&runtime, "a task that is never polled", async move {
                let _marker = marker;
                polled.store(true, Ordering::Release);
            })
        };

        let cancelling = task.cancel();

        assert!(dropped.load(Ordering::Acquire));
        // A round of the runtime, which comes to the id the task left in its queue and skips it.
        runtime.block_on(yield_now());
        assert!(!polled.load(Ordering::Acquire));
        assert_eq!(runtime.block_on(poll_immediate(cancelling)), Some(None));
    }
}

in_both_modes! {
    /// A task cancelled while it waits has its future dropped in the call, on the thread that
    /// cancels it, and the wait is over as soon as it is polled.
    fn cancelling_a_waiting_task_drops_its_future_in_the_call<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let dropped = Arc::new(AtomicBool::new(false));
        let task = {
            let marker = SetOnDrop(dropped.clone());
            M::spawn(&runtime, "a task that never finishes", async move {
                let _marker = marker;
                pending::<u32>().await
            })
        };
        // A yield is one round of the loop, in which the task is polled and left waiting.
        runtime.block_on(yield_now());
        assert!(!dropped.load(Ordering::Acquire));

        let cancelling = task.cancel();

        assert!(dropped.load(Ordering::Acquire));
        assert_eq!(block_on(poll_immediate(cancelling)), Some(None));
    }
}

in_both_modes! {
    /// A task that finished before it was cancelled hands its output back, which nobody had
    /// taken yet.
    fn cancelling_a_finished_task_hands_its_output_back<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let task = M::spawn(&runtime, "an answer", async { 42 });
        runtime.block_on(yield_now());
        assert!(task.is_finished());

        assert_eq!(block_on(poll_immediate(task.cancel())), Some(Some(42)));
    }
}

in_both_modes! {
    /// A task that panicked has no output to hand back.
    fn cancelling_a_panicked_task_hands_nothing_back<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let task = M::spawn(&runtime, "a task that panics", async {
            panic!("the task panicked on purpose");
        });
        runtime.block_on(yield_now());
        assert!(task.is_finished());

        let resolved = block_on(poll_immediate(task.cancel()));

        assert!(resolved.is_some_and(|output| output.is_none()));
    }
}

in_both_modes! {
    /// A task whose runtime went before it ended has no output to hand back, and its future is
    /// gone already, so there is nothing to wait for.
    fn cancelling_a_task_whose_runtime_is_gone_hands_nothing_back<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let task = M::spawn(&runtime, "a task that never finishes", pending::<u32>());
        runtime.block_on(yield_now());

        drop(runtime);

        assert_eq!(block_on(poll_immediate(task.cancel())), Some(None));
    }
}

in_both_modes! {
    /// A task whose output was taken by awaiting it has nothing left to hand back, and the wait
    /// is over at once rather than waiting for an outcome that has come and gone.
    fn cancelling_a_task_already_awaited_hands_nothing_back<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let mut task = M::spawn(&runtime, "an answer", async { 42 });
        assert_eq!(runtime.block_on(&mut task).unwrap(), 42);

        assert_eq!(block_on(poll_immediate(task.cancel())), Some(None));
    }
}

in_both_modes! {
    /// A future whose destructor panics as `cancel` drops it panics out of that call, as it does
    /// out of a drop of the task, and takes nothing else with it.
    fn a_panicking_destructor_panics_out_of_cancel<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let task = M::spawn(&runtime, "a task that never finishes", async {
            let _marker = PanicOnDrop;
            pending::<()>().await;
        });
        // Idle when it is cancelled, so the drop of its future is this thread's to make, and the
        // panic is the caller's to see.
        runtime.block_on(yield_now());

        assert!(catch_unwind(AssertUnwindSafe(|| task.cancel())).is_err());

        let next = M::spawn(&runtime, "the task after it", async { 7 });
        assert_eq!(runtime.block_on(next).unwrap(), 7);
    }
}

/// A task may cancel itself from inside its own poll, and await that: the wait leaves the poll
/// pending, and the future is dropped once that poll has returned, the wait with it, while the
/// runtime carries on.
#[test]
#[timeout(15000)]
fn a_local_task_may_cancel_itself() {
    let runtime = LocalRuntime::new().unwrap();
    let own = Rc::new(RefCell::new(None::<Task<()>>));
    let log = Rc::new(RefCell::new(Vec::new()));
    let dropped = Arc::new(AtomicBool::new(false));
    let task = {
        let own = own.clone();
        let log = log.clone();
        let marker = SetOnDrop(dropped.clone());
        runtime.spawn("a task that cancels itself", async move {
            let _marker = marker;
            let task = own.borrow_mut().take().expect("the task's own handle");
            let cancelling = task.cancel();
            log.borrow_mut().push("cancelled");
            cancelling.await;
            log.borrow_mut().push("resumed");
        })
    };
    *own.borrow_mut() = Some(task);

    runtime.block_on(yield_now());

    assert_eq!(*log.borrow(), ["cancelled"]);
    assert!(dropped.load(Ordering::Acquire));
    let next = runtime.spawn("the task after it", async { 7 });
    assert_eq!(runtime.block_on(next).unwrap(), 7);
}

/// A task may cancel itself from inside its own poll, and await that: the wait leaves the poll
/// pending, and the future is dropped once that poll has returned, the wait with it, while the
/// runtime carries on. Nothing of the task is left behind then: the waker the wait registered is
/// the task's own, and dropping the cancelled handle lets go of it, or the task would keep itself,
/// and the runtime's remote with it, alive for good.
#[test]
#[timeout(15000)]
fn a_shared_task_may_cancel_itself() {
    let runtime = SharedRuntime::new().unwrap();
    let remotes = Arc::strong_count(&runtime.core.remote);
    let own = Arc::new(Mutex::new(None::<Task<(), Shared>>));
    let log = Arc::new(Mutex::new(Vec::new()));
    let dropped = Arc::new(AtomicBool::new(false));
    let task = {
        let own = own.clone();
        let log = log.clone();
        let marker = SetOnDrop(dropped.clone());
        runtime.spawn("a task that cancels itself", async move {
            let _marker = marker;
            let task = own.lock().unwrap().take().expect("the task's own handle");
            let cancelling = task.cancel();
            log.lock().unwrap().push("cancelled");
            cancelling.await;
            log.lock().unwrap().push("resumed");
        })
    };
    *own.lock().unwrap() = Some(task);

    runtime.block_on(yield_now());

    assert_eq!(*log.lock().unwrap(), ["cancelled"]);
    assert!(dropped.load(Ordering::Acquire));
    assert_eq!(Arc::strong_count(&runtime.core.remote), remotes);
    let next = runtime.spawn("the task after it", async { 7 });
    assert_eq!(runtime.block_on(next).unwrap(), 7);
}

/// A task cancelled as its runtime drops it — from the destructor of its own future, here — is
/// waited for until its future is gone, though the runtime can no longer be reached to stop it:
/// the runtime is dropping that future already, and the wait lasts until it has.
#[test]
#[timeout(15000)]
fn a_task_cancelled_as_its_runtime_goes_is_waited_for_until_its_future_is_gone() {
    let runtime = LocalRuntime::new().unwrap();
    let own = Rc::new(RefCell::new(None));
    let cancelling = Rc::new(RefCell::new(None));
    let pending_at_once = Rc::new(Cell::new(None));
    let task = {
        let canceller = CancelOnDrop {
            task: own.clone(),
            cancelling: cancelling.clone(),
            pending_at_once: pending_at_once.clone(),
        };
        runtime.spawn("a task that cancels itself as it goes", async move {
            let _canceller = canceller;
            pending::<()>().await;
        })
    };
    *own.borrow_mut() = Some(task);
    runtime.block_on(yield_now());

    drop(runtime);

    // The future was still being dropped when the wait was first polled...
    assert_eq!(pending_at_once.get(), Some(true));
    // ...and is gone by now.
    let cancelling = cancelling.borrow_mut().take().unwrap();
    assert_eq!(block_on(poll_immediate(cancelling)), Some(None));
}

/// A task cancelled while another thread polls it is dropped by that thread once its poll
/// returns, and the wait for it lasts until then. Where that poll leaves the task waiting, there
/// is no output to hand back.
#[test]
#[timeout(15000)]
fn cancelling_a_task_another_thread_polls_waits_for_that_poll() {
    let runtime = SharedRuntime::new().unwrap();
    let dropped = Arc::new(AtomicBool::new(false));
    let marker = SetOnDrop(dropped.clone());
    let (task, release, driver) = held_in_its_poll(&runtime, async move {
        let _marker = marker;
        pending::<u32>().await
    });

    let mut cancelling = pin!(task.cancel());

    assert!(block_on(poll_immediate(cancelling.as_mut())).is_none());
    assert!(!dropped.load(Ordering::Acquire));
    release.send(()).unwrap();
    assert_eq!(block_on(cancelling), None);
    assert!(dropped.load(Ordering::Acquire));
    driver.join().unwrap();
}

/// A task cancelled while another thread polls it, whose poll then finishes it, hands its output
/// back once that thread has dropped its future: the task finished before the cancellation
/// reached it.
#[test]
#[timeout(15000)]
fn cancelling_a_task_another_thread_polls_to_its_end_hands_its_output_back() {
    let runtime = SharedRuntime::new().unwrap();
    let dropped = Arc::new(AtomicBool::new(false));
    let marker = SetOnDrop(dropped.clone());
    let (task, release, driver) = held_in_its_poll(&runtime, async move {
        let _marker = marker;
        42
    });

    let mut cancelling = pin!(task.cancel());

    assert!(block_on(poll_immediate(cancelling.as_mut())).is_none());
    assert!(!dropped.load(Ordering::Acquire));
    release.send(()).unwrap();
    assert_eq!(block_on(cancelling), Some(42));
    assert!(dropped.load(Ordering::Acquire));
    driver.join().unwrap();
}

/// A task cancelled while another thread polls it, whose future's destructor panics as that
/// thread drops it, ends the wait all the same: the panic is contained there, and the task is
/// still told to be over.
#[test]
#[timeout(15000)]
fn cancelling_a_task_whose_destructor_panics_on_another_thread_ends_the_wait() {
    let runtime = SharedRuntime::new().unwrap();
    let (task, release, driver) = held_in_its_poll(&runtime, async {
        let _marker = PanicOnDrop;
        pending::<u32>().await
    });

    let mut cancelling = pin!(task.cancel());

    assert!(block_on(poll_immediate(cancelling.as_mut())).is_none());
    release.send(()).unwrap();
    assert_eq!(block_on(cancelling), None);
    driver.join().unwrap();
}

/// Dropping the wait for a cancelled task lets go of the task's outcome, as dropping the task
/// does: a task that finishes in the poll it was cancelled during drops its output there, rather
/// than leave it to whoever kept a clone of its waker.
#[test]
#[timeout(15000)]
fn dropping_the_wait_for_a_cancelled_task_lets_its_output_go() {
    let runtime = SharedRuntime::new().unwrap();
    let kept = Arc::new(Mutex::new(None));
    let output_dropped = Arc::new(AtomicBool::new(false));
    let (task, release, driver) = {
        let kept = kept.clone();
        let output = SetOnDrop(output_dropped.clone());
        held_in_its_poll(&runtime, async move {
            let waker = poll_fn(|cx| Poll::Ready(cx.waker().clone())).await;
            *kept.lock().unwrap() = Some(waker);

            output
        })
    };

    drop(task.cancel());
    release.send(()).unwrap();
    driver.join().unwrap();

    assert!(kept.lock().unwrap().is_some());
    assert!(output_dropped.load(Ordering::Acquire));
}

/// The wait for a cancelled task on a shared runtime may be sent to, and awaited on, another
/// thread, as the task itself may.
#[test]
#[timeout(15000)]
fn the_wait_for_a_cancelled_shared_task_crosses_threads() {
    let runtime = SharedRuntime::new().unwrap();
    let cancelling = runtime
        .spawn("a task that never finishes", pending::<u32>())
        .cancel();

    let resolved = thread::spawn(move || block_on(cancelling)).join().unwrap();

    assert_eq!(resolved, None);
}

in_both_modes! {
    /// A wake from another thread ends the wait the thread inside `block_on` is in.
    ///
    /// Nothing to watch and no deadline, so the wait is bounded by nothing but the notification
    /// the wake writes.
    fn a_wake_from_another_thread_ends_the_wait<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let wakers = Arc::new(Mutex::new(None::<Waker>));
        let waker_thread = {
            let wakers = wakers.clone();
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(50));
                let waker = wakers.lock().unwrap_or_else(PoisonError::into_inner).take();
                if let Some(waker) = waker {
                    waker.wake();
                }
            })
        };
        let started = Instant::now();

        runtime.block_on(async {
            let mut polled = false;
            poll_fn(|cx| {
                if polled {
                    return Poll::Ready(());
                }
                polled = true;
                *wakers.lock().unwrap_or_else(PoisonError::into_inner) = Some(cx.waker().clone());

                Poll::Pending
            })
            .await
        });

        assert!(started.elapsed() >= Duration::from_millis(50));
        assert!(started.elapsed() < Duration::from_secs(5));
        waker_thread.join().unwrap();
    }
}

in_both_modes! {
    fn sleep_resolves_once_the_duration_has_passed<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let started = Instant::now();

        runtime.block_on(runtime.sleep(Duration::from_millis(50)));

        assert!(started.elapsed() >= Duration::from_millis(50));
        assert!(started.elapsed() < Duration::from_secs(5));
    }
}

in_both_modes! {
    /// A sleep shorter than a millisecond ends well before a millisecond has passed: the wait
    /// does not round its timeout up to whole milliseconds, as epoll's own timeout would, which
    /// would have every one of these sleeps take a millisecond at least. The bound leaves several
    /// times the sleep's length for a busy machine to wake the thread late.
    ///
    /// Written for Linux and Android, whose epoll takes a timeout in milliseconds; other kernels
    /// may let timers that close together go off at once, a millisecond late.
    #[cfg(any(target_os = "linux", target_os = "android"))]
    fn a_short_sleep_does_not_wait_a_whole_millisecond<M>() {
        const ROUNDS: u32 = 20;
        let runtime = Runtime::<M>::new().unwrap();
        let started = Instant::now();

        for _ in 0..ROUNDS {
            runtime.block_on(runtime.sleep(Duration::from_micros(100)));
        }

        assert!(started.elapsed() < Duration::from_millis(1) * ROUNDS);
    }
}

in_both_modes! {
    fn sleep_until_resolves_once_the_deadline_has_passed<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let started = Instant::now();
        let deadline = started + Duration::from_millis(50);

        runtime.block_on(runtime.sleep_until(deadline));

        assert!(Instant::now() >= deadline);
        assert!(started.elapsed() < Duration::from_secs(5));
    }
}

in_both_modes! {
    /// A deadline that has passed by the first poll is no reason to wait: the timer completes
    /// on that poll, with nothing left behind for the reactor to fire.
    fn sleep_until_a_passed_deadline_is_ready_on_its_first_poll<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        // The clock has moved on by the time of the poll, however little.
        let deadline = Instant::now();

        let polled = runtime.block_on(poll_immediate(runtime.sleep_until(deadline)));

        assert!(polled.is_some());
    }
}

in_both_modes! {
    /// A timer says when it comes due, whichever way it was asked for, and has no such moment
    /// where the clock cannot name one.
    fn a_sleep_reports_its_deadline<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let deadline = Instant::now() + Duration::from_secs(60);

        assert_eq!(runtime.sleep_until(deadline).deadline(), Some(deadline));
        assert!(runtime.sleep(Duration::MAX).deadline().is_none());
    }
}

in_both_modes! {
    /// A reset to a later deadline holds a timer back until that deadline, though the timer was
    /// already waiting on the earlier one.
    fn a_sleep_reset_to_a_later_deadline_resolves_at_it<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let started = Instant::now();

        runtime.block_on(async {
            let mut sleep = runtime.sleep(Duration::from_millis(10));
            // Polled first, so that the reset moves a deadline the runtime already waits on.
            assert!(poll_immediate(&mut sleep).await.is_none());
            sleep.reset_after(Duration::from_millis(50));
            sleep.await;
        });

        assert!(started.elapsed() >= Duration::from_millis(50));
        assert!(started.elapsed() < Duration::from_secs(5));
    }
}

in_both_modes! {
    /// A reset to an earlier deadline brings a waiting timer forward, rather than leaving it to
    /// the deadline it was polled with.
    fn a_sleep_reset_to_an_earlier_deadline_resolves_at_it<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let started = Instant::now();

        runtime.block_on(async {
            let mut sleep = runtime.sleep(Duration::from_secs(10));
            assert!(poll_immediate(&mut sleep).await.is_none());
            sleep.reset_after(Duration::from_millis(20));
            sleep.await;
        });

        assert!(started.elapsed() >= Duration::from_millis(20));
        assert!(started.elapsed() < Duration::from_secs(5));
    }
}

in_both_modes! {
    /// A timer that has completed completes again once reset, at its new deadline.
    fn a_completed_sleep_reset_completes_again<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let gap = Duration::from_millis(20);

        let since_the_reset = runtime.block_on(async {
            let mut sleep = runtime.sleep(Duration::from_millis(5));
            (&mut sleep).await;
            let reset = Instant::now();
            sleep.reset_after(gap);
            sleep.await;

            reset.elapsed()
        });

        assert!(since_the_reset >= gap);
        assert!(since_the_reset < Duration::from_secs(5));
    }
}

in_both_modes! {
    /// A task waiting on a timer is woken at the deadline the timer is reset to, though nothing
    /// polls the timer after the reset: the waker it was polled with moves with the deadline.
    ///
    /// That waker is one the test counts the wakes of, rather than the `block_on` call's own,
    /// which would show nothing: the call polls its future again after every wait, whatever
    /// ended it.
    fn a_reset_wakes_the_waiting_task_at_the_new_deadline<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let mut sleep = runtime.sleep(Duration::from_secs(10));
        let (counter, waker) = counting_waker();
        let mut polled = false;
        let started = Instant::now();

        runtime.block_on(poll_fn(|_| {
            if !polled {
                polled = true;
                let first = Pin::new(&mut sleep).poll(&mut Context::from_waker(&waker));
                assert!(first.is_pending());
                sleep.reset_after(Duration::from_millis(20));

                return Poll::Pending;
            }
            // The timer is never polled again, so what ends the call is the counted waker, woken
            // at the new deadline by the wake that moved with it. Had the reset let that waker
            // go instead, nothing would ever count, and once the new deadline had passed, no
            // deadline would be left to bound the wait: the call would never end.
            if counter.count() > 0 {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        }));

        assert!(started.elapsed() >= Duration::from_millis(20));
        assert!(started.elapsed() < Duration::from_secs(5));
        assert_eq!(counter.count(), 1);
    }
}

in_both_modes! {
    fn a_timeout_hands_back_what_the_future_produced<M>() {
        let runtime = Runtime::<M>::new().unwrap();

        let output = runtime.block_on(runtime.timeout(Duration::from_secs(10), async { 7 }));

        assert_eq!(output, Ok(7));
    }
}

in_both_modes! {
    fn a_timeout_on_a_future_that_never_completes_times_out<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let started = Instant::now();

        let output = runtime.block_on(runtime.timeout(Duration::from_millis(50), pending::<()>()));

        assert_eq!(output, Err(TimedOut));
        assert!(started.elapsed() >= Duration::from_millis(50));
        assert!(started.elapsed() < Duration::from_secs(5));
    }
}

in_both_modes! {
    /// A deadline that has passed by the first poll times a waiting future out on that poll,
    /// with no wait for the reactor to fire it.
    fn a_timeout_at_a_passed_deadline_times_out_on_its_first_poll<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let timeout = runtime.timeout_at(Instant::now(), pending::<()>());

        let polled = runtime.block_on(poll_immediate(timeout));

        assert_eq!(polled, Some(Err(TimedOut)));
    }
}

in_both_modes! {
    /// The future has its turn before the deadline is looked at, so one that is ready hands back
    /// its output even where the deadline has passed already.
    fn a_timeout_polls_the_future_before_the_deadline<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let timeout = runtime.timeout_at(Instant::now(), async { 7 });

        let polled = runtime.block_on(poll_immediate(timeout));

        assert_eq!(polled, Some(Ok(7)));
    }
}

in_both_modes! {
    /// A future that cannot be moved once polled, an async block that holds a borrow of its own
    /// across a wait, is run in place.
    fn a_timeout_runs_a_future_that_is_not_unpin<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let started = Instant::now();

        let output = runtime.block_on(runtime.timeout(Duration::from_secs(10), async {
            let values = [3, 5, 7];
            let last = &values[2];
            runtime.sleep(Duration::from_millis(10)).await;

            *last
        }));

        assert_eq!(output, Ok(7));
        assert!(started.elapsed() >= Duration::from_millis(10));
        assert!(started.elapsed() < Duration::from_secs(5));
    }
}

in_both_modes! {
    /// A future that ran out of time is not dropped with it: the timeout hands it back, to be
    /// driven to completion with no deadline at all.
    fn a_timeout_hands_back_its_future_after_timing_out<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        // Shut until the time-out, so that the future cannot win the race against the deadline
        // however slowly this runs.
        let open = Cell::new(false);
        let gate = Box::pin(poll_fn(|_| {
            if open.get() {
                Poll::Ready(7)
            } else {
                Poll::Pending
            }
        }));
        let mut timeout = runtime.timeout(Duration::from_millis(10), gate);

        assert_eq!(runtime.block_on(&mut timeout), Err(TimedOut));
        open.set(true);
        let output = runtime.block_on(timeout.into_inner());

        assert_eq!(output, 7);
    }
}

/// A time-out is an I/O error of the kind made for it, which carries the time-out itself, so
/// that code returning an `io::Result` can hand one on with `?`.
#[test]
#[timeout(15000)]
fn timed_out_converts_into_an_io_error_of_its_kind() {
    let error = io::Error::from(TimedOut);

    assert_eq!(error.kind(), io::ErrorKind::TimedOut);
    assert!(error.get_ref().is_some_and(|inner| inner.is::<TimedOut>()));
}

in_both_modes! {
    /// Each tick is the moment it was scheduled for, a period after the one before, the first one
    /// period after the interval was made.
    fn an_interval_ticks_once_a_period<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let period = Duration::from_millis(20);
        let started = Instant::now();

        let ticks = runtime.block_on(async {
            let mut interval = runtime.interval(period);
            [interval.tick().await, interval.tick().await, interval.tick().await]
        });

        assert!(ticks[0] >= started + period);
        assert!(ticks[0] < started + Duration::from_secs(1));
        assert_eq!(ticks[1] - ticks[0], period);
        assert_eq!(ticks[2] - ticks[1], period);
        assert!(started.elapsed() >= 3 * period);
        assert!(started.elapsed() < Duration::from_secs(5));
    }
}

in_both_modes! {
    /// An interval started at a moment that has passed already ticks on its first poll, with that
    /// moment, and keeps to the schedule from it.
    fn an_interval_at_a_passed_start_ticks_at_once<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let period = Duration::from_millis(20);
        let start = Instant::now();
        let mut interval = runtime.interval_at(start, period);

        let first = runtime.block_on(poll_immediate(interval.tick()));
        let second = runtime.block_on(interval.tick());

        assert_eq!(first, Some(start));
        assert_eq!(second, start + period);
        assert!(Instant::now() >= start + period);
    }
}

in_both_modes! {
    fn an_interval_is_a_stream_of_its_ticks<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let period = Duration::from_millis(5);
        let start = Instant::now() + period;
        let interval = runtime.interval_at(start, period);

        let ticks: Vec<Instant> = runtime.block_on(interval.take(3).collect());

        assert_eq!(ticks, [start, start + period, start + 2 * period]);
    }
}

in_both_modes! {
    /// Ticks missed while the thread was blocked are handed out one after the other, each with
    /// the moment it was scheduled for, so the schedule is kept.
    fn an_interval_bursts_through_missed_ticks<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let period = Duration::from_millis(20);

        let (first, after) = runtime.block_on(async {
            let mut interval = runtime.interval(period);
            assert_eq!(interval.missed_tick_behavior(), MissedTickBehavior::Burst);
            let first = interval.tick().await;
            // Blocked rather than waiting, so that the runtime cannot hand out a tick meanwhile.
            thread::sleep(period * 7 / 2);

            (first, [interval.tick().await, interval.tick().await, interval.tick().await])
        });

        assert_eq!(after, [first + period, first + 2 * period, first + 3 * period]);
    }
}

in_both_modes! {
    /// After a tick missed, the next one comes a full period after the late one was handed out.
    fn an_interval_delays_after_missed_ticks<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let period = Duration::from_millis(20);

        let (stalled_until, late, next) = runtime.block_on(async {
            let mut interval = runtime.interval(period);
            interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
            interval.tick().await;
            thread::sleep(period * 7 / 2);
            let stalled_until = Instant::now();

            (stalled_until, interval.tick().await, interval.tick().await)
        });

        assert!(late < stalled_until);
        assert!(next >= stalled_until + period);
        assert!(Instant::now() >= next);
    }
}

in_both_modes! {
    /// After ticks missed, the next one is the first of the original schedule still to come.
    fn an_interval_skips_missed_ticks<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let period = Duration::from_millis(20);

        let (first, stalled_until, late, next) = runtime.block_on(async {
            let mut interval = runtime.interval(period);
            interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
            let first = interval.tick().await;
            thread::sleep(period * 7 / 2);
            let stalled_until = Instant::now();

            (first, stalled_until, interval.tick().await, interval.tick().await)
        });

        // The tick that was due as the stall began is handed out late, and those missed after it
        // are not: the one after it is the first still to come once the stall was over.
        assert!(late < stalled_until);
        assert!(next > stalled_until);
        assert_eq!((late - first).as_nanos() % period.as_nanos(), 0);
        assert_eq!((next - first).as_nanos() % period.as_nanos(), 0);
    }
}

in_both_modes! {
    /// A reset starts the schedule over, one period from the reset, however close the tick it
    /// replaces was.
    fn an_interval_reset_ticks_a_period_after_it<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let period = Duration::from_millis(20);
        let mut interval = runtime.interval_at(Instant::now(), period);

        let reset = Instant::now();
        interval.reset();
        let tick = runtime.block_on(interval.tick());

        assert!(tick >= reset + period);
        assert!(Instant::now() >= tick);
        assert!(reset.elapsed() < Duration::from_secs(5));
    }
}

in_both_modes! {
    fn an_interval_with_a_zero_period_panics<M>() {
        let runtime = Runtime::<M>::new().unwrap();

        let interval = catch_unwind(AssertUnwindSafe(|| runtime.interval(Duration::ZERO)));
        let interval_at = catch_unwind(AssertUnwindSafe(|| {
            runtime.interval_at(Instant::now(), Duration::ZERO)
        }));

        assert!(interval.is_err());
        assert!(interval_at.is_err());
    }
}

in_both_modes! {
    /// An interval whose ticks the clock cannot name never ticks: one asked for a period further
    /// ahead than the clock can name has no first tick, and one started now hands out its first
    /// tick and then no other.
    ///
    /// As a stream, neither ends, and neither promises any tick to come either: the size hint of
    /// an interval is no more than any stream's.
    fn an_interval_beyond_the_clock_never_ticks<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let mut never = runtime.interval(Duration::MAX);
        let start = Instant::now();
        let mut once = runtime.interval_at(start, Duration::MAX);

        assert!(runtime.block_on(poll_immediate(never.tick())).is_none());
        assert_eq!(Stream::size_hint(&never), (0, None));
        assert_eq!(runtime.block_on(poll_immediate(once.tick())), Some(start));
        // The tick after it is a whole period after `start`, beyond the clock.
        assert!(runtime.block_on(poll_immediate(once.tick())).is_none());
        assert_eq!(Stream::size_hint(&once), (0, None));
    }
}

in_both_modes! {
    fn a_registration_reports_readiness<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (source, mut peer) = pair();
        let registration = M::register(&runtime, source.clone()).unwrap();
        // Written from another thread once the read below is waiting, so that the byte is one the
        // wait reports rather than one the first attempt to read finds for itself.
        let writer = thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));
            peer.write_all(&[7]).unwrap();
        });

        let read = runtime.block_on(read_one(&registration, &source));

        assert_eq!(read.unwrap(), 1);
        writer.join().unwrap();
    }
}

in_both_modes! {
    /// A runtime watches a descriptor through one registration at a time: a second one is turned
    /// away while the first lives, and made once it is gone.
    fn a_descriptor_is_registered_once_at_a_time<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (source, _peer) = pair();
        let registration = M::register(&runtime, source.clone()).unwrap();

        let again = M::register(&runtime, source.clone()).unwrap_err();
        assert_eq!(again.kind(), io::ErrorKind::AlreadyExists);

        drop(registration);
        M::register(&runtime, source).unwrap();
    }
}

in_both_modes! {
    /// A wait for readiness completes once another thread has written to the peer, and the read
    /// that follows it finds the byte at once, with no retry of its own.
    fn a_registration_waits_for_readiness<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (source, mut peer) = pair();
        let registration = M::register(&runtime, source.clone()).unwrap();
        // Written from another thread once the wait below is under way, so that the byte is one
        // the wait reports rather than one that was there before it.
        let writer = thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));
            peer.write_all(&[7]).unwrap();
        });

        runtime.block_on(registration.ready(Interest::Readable)).unwrap();

        assert_eq!(read_ready_byte(&source), 7);
        writer.join().unwrap();
    }
}

/// Four tasks of a local runtime wait for the readiness of one registration at once, and one
/// byte ends all four waits, leaving the byte there to be read.
#[test]
#[timeout(15000)]
fn four_local_tasks_wait_for_one_readiness_together() {
    let runtime = LocalRuntime::new().unwrap();
    let (source, mut peer) = pair();
    let registration = Rc::new(runtime.register(source.clone()).unwrap());
    let tasks: Vec<_> = (0..4)
        .map(|_| {
            let registration = registration.clone();
            runtime.spawn("a task waiting for readiness", async move {
                registration.ready(Interest::Readable).await.unwrap();
            })
        })
        .collect();
    // Written from another thread once the four tasks are waiting.
    let writer = thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        peer.write_all(&[7]).unwrap();
    });

    runtime.block_on(async {
        for task in tasks {
            task.await.unwrap();
        }
    });

    // Waiting for readiness took nothing from the source.
    let read = runtime.block_on(read_one(&registration, &source));
    assert_eq!(read.unwrap(), 1);
    writer.join().unwrap();
}

/// Four tasks of a shared runtime wait for the readiness of one registration at once, and one
/// byte ends all four waits, leaving the byte there to be read.
#[test]
#[timeout(15000)]
fn four_shared_tasks_wait_for_one_readiness_together() {
    let runtime = SharedRuntime::new().unwrap();
    let (source, mut peer) = pair();
    let registration = Arc::new(runtime.register(source.clone()).unwrap());
    let tasks: Vec<_> = (0..4)
        .map(|_| {
            let registration = registration.clone();
            runtime.spawn("a task waiting for readiness", async move {
                registration.ready(Interest::Readable).await.unwrap();
            })
        })
        .collect();
    // Written from another thread once the four tasks are waiting.
    let writer = thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        peer.write_all(&[7]).unwrap();
    });

    runtime.block_on(async {
        for task in tasks {
            task.await.unwrap();
        }
    });

    // Waiting for readiness took nothing from the source.
    let read = runtime.block_on(read_one(&registration, &source));
    assert_eq!(read.unwrap(), 1);
    writer.join().unwrap();
}

in_both_modes! {
    /// A wait for readiness is not completed by the wake of a timer it shares its task with.
    ///
    /// The timer wakes the task, which polls both futures again, and the wait for readiness has
    /// to take that for what it is: a poll the source did not cause. Nothing is written to the
    /// peer, which is kept so that the source does not become readable by its end.
    fn a_readiness_wait_is_not_completed_by_a_timer<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (source, _peer) = pair();
        let registration = M::register(&runtime, source).unwrap();
        let started = Instant::now();

        let winner = runtime.block_on(or(
            async {
                registration.ready(Interest::Readable).await.unwrap();
                "ready"
            },
            async {
                runtime.sleep(Duration::from_millis(20)).await;
                "timer"
            },
        ));

        assert_eq!(winner, "timer");
        assert!(started.elapsed() >= Duration::from_millis(20));
    }
}

in_both_modes! {
    /// A wait for readiness dropped while it waits, whether by a poll that gave up on it or by
    /// the race it lost, leaves the runtime able to finish its `block_on` and the registration
    /// able to wait again.
    fn a_dropped_readiness_wait_leaves_the_registration_usable<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (source, mut peer) = pair();
        let registration = M::register(&runtime, source.clone()).unwrap();

        // Dropped after a single poll, which left it waiting.
        let polled = runtime.block_on(poll_immediate(registration.ready(Interest::Readable)));
        assert!(polled.is_none());
        // Dropped by the race it lost, when the timer won.
        let winner = runtime.block_on(or(
            async {
                registration.ready(Interest::Readable).await.unwrap();
                "ready"
            },
            async {
                runtime.sleep(Duration::from_millis(20)).await;
                "timer"
            },
        ));
        assert_eq!(winner, "timer");
        // Written from another thread once the wait below is under way.
        let writer = thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));
            peer.write_all(&[7]).unwrap();
        });

        runtime.block_on(registration.ready(Interest::Readable)).unwrap();

        assert_eq!(read_ready_byte(&source), 7);
        writer.join().unwrap();
    }
}

in_both_modes! {
    /// A socket of one protocol family is watched beside a wake channel of another.
    ///
    /// Winsock takes every socket of one `select` call to come from a single service provider,
    /// and a runtime's wake channel is a loopback TCP connection while a caller may register an
    /// `AF_UNIX` socket directly. A wait therefore holds sockets of both families, and this is
    /// where that mixture is asked for: an `AF_UNIX` socket registered on a runtime is reported
    /// readable just as a socket of the wake channel's own family is.
    #[cfg(windows)]
    fn an_af_unix_socket_is_watched_beside_the_tcp_wake_socket<M>() {
        use std::{
            env, fs,
            os::windows::io::{AsSocket, OwnedSocket},
            process,
            time::SystemTime,
        };

        use uds_windows::{UnixListener, UnixStream};

        let runtime = Runtime::<M>::new().unwrap();
        // Named after this process, this moment and the flavour under test, so that two runs of
        // the suite, or the two flavours of this test running side by side, cannot meet over one
        // path.
        let since_epoch = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        let path = env::temp_dir().join(format!(
            "zruntime-{}-{}-{}.sock",
            process::id(),
            since_epoch.as_nanos(),
            std::any::type_name::<M>().rsplit("::").next().unwrap_or_default(),
        ));
        let listener = UnixListener::bind(&path).unwrap();
        let client = UnixStream::connect(&path).unwrap();
        let (mut server, _) = listener.accept().unwrap();
        client.set_nonblocking(true).unwrap();
        server.set_nonblocking(true).unwrap();
        // The stream keeps the handle it was made with, so what the source is given is a
        // duplicate of it.
        let owned: OwnedSocket = client.as_socket().try_clone_to_owned().unwrap();
        let source: TestSource = Arc::new(owned);
        let registration = M::register(&runtime, source.clone()).unwrap();
        // Written from another thread once the read below is waiting, so that the byte is one the
        // wait reports rather than one the first attempt to read finds for itself.
        let writer = thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));
            server.write_all(&[7]).unwrap();
        });

        let read = runtime.block_on(read_one(&registration, &source));

        assert_eq!(read.unwrap(), 1);
        writer.join().unwrap();
        drop(listener);
        fs::remove_file(&path).unwrap();
    }
}

in_both_modes! {
    /// A runtime that goes with tasks still on it drops their futures and fails their handles,
    /// whether they were polled before it went or never were.
    fn dropping_the_runtime_fails_pending_handles<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let polled = M::spawn(&runtime, "a task that never finishes", pending::<()>());
        runtime.block_on(yield_now());
        let dropped = Arc::new(AtomicBool::new(false));
        let never_polled = {
            let marker = SetOnDrop(dropped.clone());
            M::spawn(&runtime, "a task that is never polled", async move {
                let _marker = marker;
            })
        };

        drop(runtime);

        assert!(dropped.load(Ordering::Acquire));
        for task in [polled, never_polled] {
            let error = futures::executor::block_on(task).unwrap_err();
            assert_eq!(error.to_string(), "the task's runtime is gone");
        }
    }
}

in_both_modes! {
    /// A `block_on` from a thread that is driving a runtime already panics rather than hangs:
    /// from inside the future of a `block_on` on the same runtime, or on another runtime of
    /// either flavour. The call that panics leaves the one it was made from driving as before.
    fn a_nested_block_on_panics<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let local = LocalRuntime::new().unwrap();
        let shared = SharedRuntime::new().unwrap();

        let nested = runtime.block_on(async {
            let panicked = [
                catch_unwind(AssertUnwindSafe(|| runtime.block_on(async {}))).is_err(),
                catch_unwind(AssertUnwindSafe(|| local.block_on(async {}))).is_err(),
                catch_unwind(AssertUnwindSafe(|| shared.block_on(async {}))).is_err(),
            ];
            // Still driving: a task and a timer spawned after the panics run as ever.
            let task = M::spawn(&runtime, "a task after the panics", async { 7 });
            runtime.sleep(Duration::from_millis(1)).await;

            (panicked, task.await.unwrap())
        });

        assert_eq!(nested, ([true; 3], 7));
        // And the thread drives each of them in turn once it has left the first.
        assert_eq!(local.block_on(async { 7 }), 7);
        assert_eq!(shared.block_on(async { 7 }), 7);
    }
}

in_both_modes! {
    /// The runtime a thread drives is on record for as long as the thread is inside `block_on` on
    /// it, and not a moment longer, whether the call returns or unwinds.
    fn the_runtime_a_thread_drives_is_on_record_while_it_is_inside_block_on<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        assert!(M::driven().is_none());

        runtime.block_on(async {
            let driven = M::driven().expect("the runtime this thread drives");
            assert!(std::ptr::eq(&*driven, &*runtime.core));
        });
        assert!(M::driven().is_none());

        let panicked = catch_unwind(AssertUnwindSafe(|| {
            runtime.block_on(async { panic!("a future that panics") });
        }));
        assert!(panicked.is_err());
        assert!(M::driven().is_none());
    }
}

in_both_modes! {
    /// A `block_on` whose future panics leaves the runtime to be driven again, by this thread or
    /// another.
    fn a_panic_in_the_future_leaves_the_runtime_drivable<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let panicked = catch_unwind(AssertUnwindSafe(|| {
            runtime.block_on(async { panic!("a future that panics") });
        }));
        assert!(panicked.is_err());

        let task = M::spawn(&runtime, "the task after it", async { 7 });
        assert_eq!(runtime.block_on(task).unwrap(), 7);
    }
}

/// A local runtime runs futures that are not `Send`: a task can hold an `Rc` and borrow a
/// `RefCell` across its awaits.
#[test]
#[timeout(15000)]
fn a_local_task_can_hold_an_rc() {
    let runtime = LocalRuntime::new().unwrap();
    let log = Rc::new(RefCell::new(Vec::new()));
    let task = {
        let log = log.clone();
        let sleeper = runtime.clone();
        runtime.spawn("a task holding an `Rc`", async move {
            log.borrow_mut().push("before");
            sleeper.sleep(Duration::from_millis(1)).await;
            log.borrow_mut().push("after");

            Rc::strong_count(&log)
        })
    };

    let count = runtime.block_on(task).unwrap();

    assert_eq!(count, 2);
    assert_eq!(*log.borrow(), ["before", "after"]);
}

/// A thread of its own wakes a local task through the task's waker, and breaks the wait the
/// thread inside `block_on` is in to do so.
///
/// A waker is `Send` whatever the task it wakes is, so a local task waiting on a thread's work is
/// woken by that thread as any other task is.
#[cfg(feature = "event")]
#[test]
#[timeout(15000)]
fn a_thread_wakes_a_local_task() {
    let runtime = LocalRuntime::new().unwrap();
    let event = Arc::new(Event::new());
    let listener = event.listen();
    let local = Rc::new(7);
    let task = runtime.spawn("a local task waiting on a thread", async move {
        listener.await;
        *local
    });
    let notifier = thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        event.notify(1);
    });
    let started = Instant::now();

    assert_eq!(runtime.block_on(task).unwrap(), 7);

    assert!(started.elapsed() >= Duration::from_millis(50));
    assert!(started.elapsed() < Duration::from_secs(5));
    notifier.join().unwrap();
}

/// A task spawned from another thread onto a shared runtime breaks the wait of the thread inside
/// `block_on` on it, which then runs the task.
#[cfg(feature = "event")]
#[test]
#[timeout(15000)]
fn a_spawn_from_another_thread_reaches_the_driving_thread() {
    let runtime = SharedRuntime::new().unwrap();
    let ran = Arc::new(Event::new());
    // Taken before the thread starts, so that the announcement cannot be missed.
    let announced = ran.listen();
    let spawner = {
        let runtime = runtime.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));
            runtime
                .spawn("a task spawned from another thread", async move {
                    ran.notify(1);
                })
                .detach();
        })
    };

    runtime.block_on(announced);

    spawner.join().unwrap();
}

/// A shared runtime is driven by one thread at a time, so a second thread asking to drive it
/// while the first does is told so rather than left to race it.
#[cfg(feature = "event")]
#[test]
#[timeout(15000)]
fn a_second_concurrent_block_on_panics() {
    let runtime = SharedRuntime::new().unwrap();
    let inside = Arc::new(Event::new());
    let release = Arc::new(Event::new());
    // Both taken before the thread starts, so that neither notification can be missed.
    let announced = inside.listen();
    let released = release.listen();
    let driver = {
        let runtime = runtime.clone();
        thread::spawn(move || {
            runtime.block_on(async move {
                inside.notify(1);
                released.await;
            })
        })
    };
    futures::executor::block_on(announced);

    let panicked = catch_unwind(AssertUnwindSafe(|| runtime.block_on(async {}))).unwrap_err();

    let message = panicked
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| panicked.downcast_ref::<String>().map(String::as_str))
        .unwrap_or_default();
    assert!(message.contains("another thread is driving"), "{message}");
    release.notify(1);
    driver.join().unwrap();
    // And once the first thread has left, this one may drive it.
    assert_eq!(runtime.block_on(async { 7 }), 7);
}

/// A task spawned with no runtime to name goes on the local runtime the thread drives, whether it
/// is spawned from the future of `block_on` or from inside a task of that runtime, and not on any
/// other runtime there is. The futures hold an `Rc`, which only a local runtime takes.
#[test]
#[timeout(15000)]
fn spawn_local_puts_the_task_on_the_runtime_the_thread_drives() {
    let runtime = LocalRuntime::new().unwrap();
    let other = LocalRuntime::new().unwrap();

    let (from_block_on, from_a_task) = runtime.block_on(async {
        let local = Rc::new(7);
        let task = crate::spawn_local({
            let local = local.clone();
            async move { *local }
        });
        // Queued here, on this runtime and on no other, before it has had a turn.
        assert_eq!(runtime.core.scheduler.live_tasks(), 1);
        assert_eq!(other.core.scheduler.live_tasks(), 0);
        let from_block_on = task.await.unwrap();

        let spawning = crate::spawn_local({
            let runtime = runtime.clone();
            async move {
                let spawned = crate::spawn_local(async move { *local });
                // This task and the one it spawned, both on the runtime that is running them.
                assert_eq!(runtime.core.scheduler.live_tasks(), 2);

                spawned.await.unwrap()
            }
        });

        (from_block_on, spawning.await.unwrap())
    });

    assert_eq!((from_block_on, from_a_task), (7, 7));
    assert_eq!(other.core.scheduler.live_tasks(), 0);
}

/// A task spawned with no runtime to name goes on the shared runtime the thread drives, whether
/// the runtime was made by `Runtime::new` or not, from the future of `block_on` and from inside a
/// task of that runtime alike, and runs on the thread inside `block_on` as every task of it does.
#[test]
#[timeout(15000)]
fn spawn_puts_the_task_on_the_runtime_the_thread_drives() {
    let runtime = SharedRuntime::new().unwrap();
    let other = SharedRuntime::new().unwrap();

    let (from_block_on, from_a_task) = runtime.block_on(async {
        let task = crate::spawn(async { thread::current().id() });
        // Queued here, on this runtime and on no other, before it has had a turn.
        assert_eq!(runtime.core.scheduler.live_tasks(), 1);
        assert_eq!(other.core.scheduler.live_tasks(), 0);
        let from_block_on = task.await.unwrap();

        let spawning = crate::spawn({
            let runtime = runtime.clone();
            async move {
                let spawned = crate::spawn(async { thread::current().id() });
                // This task and the one it spawned, both on the runtime that is running them.
                assert_eq!(runtime.core.scheduler.live_tasks(), 2);

                spawned.await.unwrap()
            }
        });

        (from_block_on, spawning.await.unwrap())
    });

    assert_eq!(from_block_on, thread::current().id());
    assert_eq!(from_a_task, thread::current().id());
    assert_eq!(other.core.scheduler.live_tasks(), 0);
}

/// A local task has no runtime to go on where the thread drives no local runtime: outside any
/// `block_on`, and inside the `block_on` of a shared runtime, whose tasks may run on any thread. A
/// call that panics leaves the runtime it was made inside driving as before.
#[test]
#[timeout(15000)]
fn spawn_local_panics_where_the_thread_drives_no_local_runtime() {
    let outside = catch_unwind(AssertUnwindSafe(|| crate::spawn_local(async {}))).unwrap_err();
    assert!(panic_message(&*outside).contains("no local runtime"));

    let shared = SharedRuntime::new().unwrap();
    let (message, next) = shared.block_on(async {
        let inside = catch_unwind(AssertUnwindSafe(|| crate::spawn_local(async {}))).unwrap_err();
        // Still driving: a task spawned after the panic runs as ever.
        let next = crate::spawn(async { 7 });

        (panic_message(&*inside).to_owned(), next.await.unwrap())
    });

    assert!(message.contains("no local runtime"), "{message}");
    assert_eq!(next, 7);
}

/// Without the `helper` feature, which would give a thread that drives no shared runtime the
/// process's to spawn on, there is none for a shared task to go on there: outside any `block_on`,
/// and inside the `block_on` of a local runtime.
#[cfg(not(feature = "helper"))]
#[test]
#[timeout(15000)]
fn spawn_panics_where_the_thread_drives_no_shared_runtime() {
    let outside = catch_unwind(AssertUnwindSafe(|| crate::spawn(async {}))).unwrap_err();
    assert!(panic_message(&*outside).contains("no shared runtime"));

    let local = LocalRuntime::new().unwrap();
    let (message, next) = local.block_on(async {
        let inside = catch_unwind(AssertUnwindSafe(|| crate::spawn(async {}))).unwrap_err();
        // Still driving: a task spawned after the panic runs as ever.
        let next = crate::spawn_local(async { 7 });

        (panic_message(&*inside).to_owned(), next.await.unwrap())
    });

    assert!(message.contains("no shared runtime"), "{message}");
    assert_eq!(next, 7);
}

/// A task spawned with no runtime to name, inside a `block_on` run from the destructor of a
/// thread-local as its thread ends, goes on the runtime that call drives, as it does anywhere
/// else: a local runtime, and a shared one made by `Runtime::new`, whose task the thread inside
/// the call runs.
///
/// The thread drives a runtime of each flavour once the value is in place, so that the record of
/// the runtime it drives is first reached after that value: thread-local destructors run in
/// reverse order of registration, and a record with a destructor of its own would be gone by the
/// time the value's runs. A panic out of a destructor there is an abort, so the destructor catches
/// what panics and hands it on.
#[test]
#[timeout(15000)]
fn a_spawn_inside_a_block_on_as_a_thread_ends_goes_on_the_runtime_it_drives() {
    /// What a spawn inside a `block_on` of each flavour came to: the local task's output, and
    /// whether the shared task ran on the thread inside the call, or what panicked instead.
    type Spawned = (Result<u32, String>, Result<bool, String>);

    /// Spawns inside a `block_on` of each flavour as it is dropped, and sends what came of it.
    struct SpawnsAsItGoes(Option<mpsc::Sender<Spawned>>);

    impl Drop for SpawnsAsItGoes {
        fn drop(&mut self) {
            let Some(spawned) = self.0.take() else {
                return;
            };
            DROPPING.with(|dropping| dropping.set(true));
            let local = catch_unwind(|| {
                LocalRuntime::new()
                    .unwrap()
                    .block_on(async { crate::spawn_local(async { 7 }).await.unwrap() })
            });
            let shared = catch_unwind(|| {
                SharedRuntime::new().unwrap().block_on(async {
                    crate::spawn(async { DROPPING.with(Cell::get) })
                        .await
                        .unwrap()
                })
            });
            let message = |panic: Box<dyn Any + Send>| panic_message(&*panic).to_owned();
            let _ = spawned.send((local.map_err(message), shared.map_err(message)));
        }
    }

    thread_local! {
        static SPAWNS_AS_IT_GOES: RefCell<SpawnsAsItGoes> =
            const { RefCell::new(SpawnsAsItGoes(None)) };

        /// Whether this thread is the one dropping the value above, for a task to tell which
        /// thread runs it. With no destructor, so that it is there to read as the thread ends.
        static DROPPING: Cell<bool> = const { Cell::new(false) };
    }

    let (sender, spawned) = mpsc::channel();
    let thread = thread::spawn(move || {
        SPAWNS_AS_IT_GOES.with(|spawns| spawns.borrow_mut().0 = Some(sender));
        LocalRuntime::new().unwrap().block_on(async {});
        SharedRuntime::new().unwrap().block_on(async {});
    });

    assert!(thread.join().is_ok());
    // The shared task run by the thread dropping the value, inside the call, rather than by a
    // helper thread of some other runtime. Both compared at once, so that a failure shows both.
    assert_eq!(spawned.recv().unwrap(), (Ok(7), Ok(true)));
}

/// A task spawned with no name is named by the call that spawned it, as its `Debug` shows, while a
/// name given prints as the string it is.
#[test]
#[timeout(15000)]
fn a_task_spawned_with_no_name_is_named_by_where_it_was_spawned() {
    let local = LocalRuntime::new().unwrap();
    let shared = SharedRuntime::new().unwrap();

    let (local_task, local_line) =
        local.block_on(async { (crate::spawn_local(async {}), line!()) });
    let (shared_task, shared_line) = shared.block_on(async { (crate::spawn(async {}), line!()) });
    let named = local.spawn("socket reader", async {});

    // The line is the one of the call, and the column is left to the formatter.
    assert!(
        format!("{local_task:?}").contains(&format!("task: spawned at {}:{local_line}:", file!())),
        "{local_task:?}"
    );
    assert!(
        format!("{shared_task:?}")
            .contains(&format!("task: spawned at {}:{shared_line}:", file!())),
        "{shared_task:?}"
    );
    assert_eq!(
        format!("{named:?}"),
        r#"Task(JoinHandle { task: "socket reader", detached: false, .. })"#
    );
}

/// What a shared runtime's handles are shared as: a task handle and a registration live in state
/// that several threads reach, a runtime handle is cloned into the tasks it spawns, and a timer,
/// a timeout or an interval goes wherever the task awaiting it is polled.
#[test]
fn shared_handles_cross_threads() {
    fn sent_and_shared<T>()
    where
        T: Send + Sync + Unpin,
    {
    }

    sent_and_shared::<SharedRuntime>();
    sent_and_shared::<Task<(), Shared>>();
    sent_and_shared::<Registration<Shared>>();
    sent_and_shared::<Sleep<Shared>>();
    sent_and_shared::<Timeout<Pending<()>, Shared>>();
    sent_and_shared::<Interval<Shared>>();
}

/// Spawning and registering, through whichever of the two flavours' own methods `Self` names.
///
/// The two take different bounds — a shared runtime's tasks and sources have to be `Send` — so
/// this takes the stricter of each, which the tests written for both flavours meet.
trait TestMode: Mode {
    /// Spawns `future` on `runtime`.
    fn spawn<T>(
        runtime: &Runtime<Self>,
        name: &'static str,
        future: impl Future<Output = T> + Send + 'static,
    ) -> Task<T, Self>
    where
        T: Send + 'static;

    /// Registers `source` on `runtime`.
    fn register(runtime: &Runtime<Self>, source: TestSource) -> io::Result<Registration<Self>>;
}

impl TestMode for Local {
    fn spawn<T>(
        runtime: &LocalRuntime,
        name: &'static str,
        future: impl Future<Output = T> + Send + 'static,
    ) -> Task<T, Local>
    where
        T: Send + 'static,
    {
        runtime.spawn(name, future)
    }

    fn register(runtime: &LocalRuntime, source: TestSource) -> io::Result<Registration<Local>> {
        runtime.register(source)
    }
}

impl TestMode for Shared {
    fn spawn<T>(
        runtime: &SharedRuntime,
        name: &'static str,
        future: impl Future<Output = T> + Send + 'static,
    ) -> Task<T, Shared>
    where
        T: Send + 'static,
    {
        runtime.spawn(name, future)
    }

    fn register(runtime: &SharedRuntime, source: TestSource) -> io::Result<Registration<Shared>> {
        runtime.register(source)
    }
}

/// Reads one byte from `source` through `registration`, the way a caller reads its socket.
async fn read_one<M>(registration: &Registration<M>, source: &TestSource) -> io::Result<usize>
where
    M: Mode,
{
    poll_fn(|cx| {
        let mut byte = [MaybeUninit::<u8>::uninit(); 1];

        registration.poll_io(cx, Interest::Readable, || {
            SockRef::from(source).recv(&mut byte)
        })
    })
    .await
}

/// Reads the one byte `source` has right now, with no waiting and no retry, the way a caller does
/// that has just been told the source is ready: a `WouldBlock` here fails the test.
fn read_ready_byte(source: &TestSource) -> u8 {
    let mut byte = [0];
    let socket = SockRef::from(source);
    assert_eq!((&*socket).read(&mut byte).unwrap(), 1);

    byte[0]
}

/// The message a panic carried, which is a string for a panic made from a string literal or a
/// format string, as every one this crate makes is.
fn panic_message(payload: &(dyn Any + Send)) -> &str {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or_default()
}

/// Sets the flag it holds when it is dropped.
struct SetOnDrop(Arc<AtomicBool>);

impl Drop for SetOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

/// Panics when it is dropped.
struct PanicOnDrop;

impl Drop for PanicOnDrop {
    fn drop(&mut self) {
        panic!("dropped with a panic on purpose");
    }
}

/// A future that is ready at once with the value it holds, and panics when it is dropped.
struct ReadyThenPanicOnDrop(u32);

impl Future for ReadyThenPanicOnDrop {
    type Output = u32;

    fn poll(self: std::pin::Pin<&mut Self>, _cx: &mut std::task::Context<'_>) -> Poll<u32> {
        Poll::Ready(self.0)
    }
}

impl Drop for ReadyThenPanicOnDrop {
    fn drop(&mut self) {
        panic!("dropped with a panic on purpose");
    }
}

/// Spawns on `runtime`, through `spawn`, a task whose future notes whether its own task is
/// finished as the future is dropped, and has the task end as `end` says.
///
/// Gives what the future noted, `None` where it noted nothing, after checking that the task is
/// finished once it has ended.
fn finished_as_future_drops<M>(
    runtime: Runtime<M>,
    end: End,
    spawn: impl FnOnce(&Runtime<M>, NoteFinishedOnDrop<M>) -> Task<(), M>,
) -> Option<bool>
where
    M: Mode,
{
    let slot = Arc::new(Mutex::new(None));
    let noted = Arc::new(Mutex::new(None));
    let task = spawn(
        &runtime,
        NoteFinishedOnDrop {
            ready: matches!(end, End::Completes),
            task: slot.clone(),
            noted: noted.clone(),
        },
    );
    *slot.lock().unwrap() = Some(task);

    match end {
        End::Completes => runtime.block_on(yield_now()),
        End::GoesWithItsRuntimePolled => {
            runtime.block_on(yield_now());
            drop(runtime);
        }
        End::GoesWithItsRuntimeUnpolled => drop(runtime),
    }

    assert!(slot.lock().unwrap().as_ref().unwrap().is_finished());

    *noted.lock().unwrap()
}

/// How the task [`finished_as_future_drops`] spawns comes to its end.
#[derive(Clone, Copy, Debug)]
enum End {
    /// Its future is ready on its first poll.
    Completes,
    /// Its future is left waiting by its first poll, and then goes with the runtime.
    GoesWithItsRuntimePolled,
    /// Its future goes with the runtime before it is ever polled.
    GoesWithItsRuntimeUnpolled,
}

impl End {
    const ALL: [Self; 3] = [
        Self::Completes,
        Self::GoesWithItsRuntimePolled,
        Self::GoesWithItsRuntimeUnpolled,
    ];
}

/// A future that is ready at once where `ready`, and waits for ever otherwise, and that notes
/// whether the task it belongs to is finished when it is dropped.
///
/// The task is kept in a slot of the test's own, which the future reaches it through.
struct NoteFinishedOnDrop<M>
where
    M: Mode,
{
    ready: bool,
    task: Arc<Mutex<Option<Task<(), M>>>>,
    noted: Arc<Mutex<Option<bool>>>,
}

impl<M> Future for NoteFinishedOnDrop<M>
where
    M: Mode,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        if self.ready {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

impl<M> Drop for NoteFinishedOnDrop<M>
where
    M: Mode,
{
    fn drop(&mut self) {
        let finished = self.task.lock().unwrap().as_ref().map(Task::is_finished);
        *self.noted.lock().unwrap() = finished;
    }
}

/// Cancels the task in `task` as it is dropped, polls the wait for it once, and leaves that wait
/// in `cancelling`, noting in `pending_at_once` whether that poll left it pending.
///
/// What it finds goes into the test's own cells rather than into assertions of its own: a
/// destructor the runtime runs has its panics contained, and an assertion failing there would
/// fail nothing.
struct CancelOnDrop {
    task: Rc<RefCell<Option<Task<()>>>>,
    cancelling: Rc<RefCell<Option<Cancelling>>>,
    pending_at_once: Rc<Cell<Option<bool>>>,
}

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        let Some(task) = self.task.borrow_mut().take() else {
            return;
        };
        let mut cancelling = Box::pin(task.cancel());
        let polled = cancelling
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()));
        self.pending_at_once.set(Some(polled.is_pending()));
        *self.cancelling.borrow_mut() = Some(cancelling);
    }
}

/// The wait for a cancelled local task, boxed so that a test can keep it.
type Cancelling = Pin<Box<dyn Future<Output = Option<()>>>>;

/// Spawns `future` on `runtime` behind a first poll that holds the thread making it until the
/// sender handed back is sent to or dropped, and has a thread of its own make that poll, inside a
/// `block_on` on `runtime` that lasts one round of its loop.
///
/// Returns once that poll has begun, with the task, that sender, and that thread, which leaves
/// `block_on` once the poll has returned and the scheduler has done with the task what the poll
/// left it to do.
fn held_in_its_poll<F>(
    runtime: &SharedRuntime,
    future: F,
) -> (
    Task<F::Output, Shared>,
    mpsc::Sender<()>,
    thread::JoinHandle<()>,
)
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let (announce, announced) = mpsc::channel();
    let (release, released) = mpsc::channel();
    let task = runtime.spawn("a task held in its poll", async move {
        announce.send(()).unwrap();
        // A sender dropped by a test that failed first lets the poll go on as well.
        let _ = released.recv();
        future.await
    });
    let driver = {
        let runtime = runtime.clone();
        // A yield is one round of the loop, in which the task is polled once.
        thread::spawn(move || runtime.block_on(yield_now()))
    };
    announced.recv().unwrap();

    (task, release, driver)
}

/// A counter and the waker that counts into it.
pub(super) fn counting_waker() -> (Arc<Counter>, Waker) {
    let counter = Arc::new(Counter::default());
    let waker = Waker::from(counter.clone());

    (counter, waker)
}

/// A waker that counts how often it has been woken.
#[derive(Default)]
pub(super) struct Counter(AtomicUsize);

impl Counter {
    /// How often this waker has been woken.
    pub(super) fn count(&self) -> usize {
        self.0.load(Ordering::Acquire)
    }
}

impl Wake for Counter {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.fetch_add(1, Ordering::AcqRel);
    }
}

/// A source shareable between the registration a test makes and the reads or writes it makes
/// directly: an owned descriptor of its own, behind an `Arc` of the test's own.
#[cfg(unix)]
pub(super) type TestSource = Arc<std::os::fd::OwnedFd>;
#[cfg(windows)]
pub(super) type TestSource = Arc<std::os::windows::io::OwnedSocket>;

/// A connected pair: the source to register, and the far end to drive it from.
pub(super) fn pair() -> (TestSource, Socket) {
    let (near, far) = connected();
    near.set_nonblocking(true).unwrap();
    far.set_nonblocking(true).unwrap();

    (owned_source(near), far)
}

/// Wraps `socket` as a [`TestSource`]: one clone goes to the registration, the other is kept
/// for the reads and writes the tests make directly.
#[cfg(unix)]
fn owned_source(socket: Socket) -> TestSource {
    let owned: std::os::fd::OwnedFd = socket.into();

    Arc::new(owned)
}

/// Wraps `socket` as a [`TestSource`]: one clone goes to the registration, the other is kept
/// for the reads and writes the tests make directly.
#[cfg(windows)]
fn owned_source(socket: Socket) -> TestSource {
    let owned: std::os::windows::io::OwnedSocket = socket.into();

    Arc::new(owned)
}

/// Two sockets connected to one another.
#[cfg(unix)]
fn connected() -> (Socket, Socket) {
    Socket::pair(socket2::Domain::UNIX, socket2::Type::STREAM, None).unwrap()
}

/// Two sockets connected to one another.
///
/// Winsock has no socket pair, so this is a loopback connection which a listener of its own
/// accepts and then has no further use for.
#[cfg(windows)]
fn connected() -> (Socket, Socket) {
    use std::net::{TcpListener, TcpStream};

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let far = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let near = loop {
        let (accepted, _) = listener.accept().unwrap();
        // A loopback listener is reachable by anything else on the machine, so a connection that
        // is not the one made just above is turned away rather than taken for it.
        if accepted.peer_addr().unwrap() == far.local_addr().unwrap() {
            break accepted;
        }
    };

    // One-byte messages travel this pair, as they do the poller's wake pair, so neither end
    // holds a send back for the peer's acknowledgement of the one before it.
    near.set_nodelay(true).unwrap();
    far.set_nodelay(true).unwrap();

    (near.into(), far.into())
}
