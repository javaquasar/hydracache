//! Tests of `zruntime::unblock`, the blocking work it hands to a pool of threads, and the future
//! it hands back.
//!
//! The work needs no runtime, so these drive the future with the `block_on` of `futures`, or
//! poll it by hand with wakers of their own. They come in the order of what they pin down: that
//! the future hands back the value the work returned, that the work starts without the future
//! being polled, that a panic in the work reaches whoever awaits the future, which thread the work
//! runs on, that the future prints without its value, and, with wakers that show who still holds
//! them, that a future dropped while the work goes on takes back the waker it left for the thread,
//! and that the thread has let go of the waker before the future it woke can resolve.
//!
//! How the pool shares its threads out among the work handed to it is tested in the `pool`
//! module, on pools of its own, small enough to fill.

use std::{
    future::Future,
    panic::catch_unwind,
    pin::Pin,
    sync::{Arc, mpsc},
    task::{Context, Poll, Wake, Waker},
    thread,
    time::Duration,
};

use futures::executor::block_on;
use ntest::timeout;

use crate::unblock;

mod io;
mod pool;

/// The future resolves to the value the work returned.
#[test]
#[timeout(15000)]
fn the_work_hands_its_value_back() {
    assert_eq!(block_on(unblock(|| 42)), 42);
}

/// The work goes to the pool as it is handed over, not as the future is first polled: the work
/// runs although nothing has polled the future, and nothing ever does.
#[test]
#[timeout(15000)]
fn the_work_starts_without_the_future_being_polled() {
    let (report, reported) = mpsc::channel();
    let future = unblock(move || report.send(()).expect("the test waits for the work"));

    reported.recv().expect("the work runs without a poll");
    drop(future);
}

/// A panic in the work is raised again where the future is polled, with the payload it had, so
/// it reaches whoever awaits the future.
#[test]
#[timeout(15000)]
fn a_panic_in_the_work_reaches_the_caller() {
    let panicked = catch_unwind(|| block_on(unblock(|| panic!("the blocking work panicked"))))
        .expect_err("awaiting work that panicked panics in turn");

    assert_eq!(
        panicked.downcast_ref::<&str>(),
        Some(&"the blocking work panicked"),
    );
}

/// The work runs on a thread of the pool, named for what it does.
#[test]
#[timeout(15000)]
fn the_work_runs_on_a_thread_named_for_it() {
    let name = block_on(unblock(|| thread::current().name().map(ToOwned::to_owned)));

    assert_eq!(name.as_deref(), Some("zruntime blocking work"));
}

/// A future prints without the value it resolves to, so that value need not be `Debug`.
#[test]
#[timeout(15000)]
fn a_future_prints_without_its_value() {
    struct NotDebug;

    let future = unblock(|| NotDebug);

    assert_eq!(format!("{future:?}"), "BlockingWork { .. }");
    // Awaited, so that the thread is done with the work by the time the test ends.
    block_on(future);
}

/// A future dropped while the work goes on takes back the waker it left for the thread, so that
/// nothing of the task that waited stays with the thread until the work is over.
#[test]
#[timeout(15000)]
fn a_dropped_future_takes_its_waker_back_while_the_work_runs_on() {
    let probe = Arc::new(Unpark(thread::current()));
    let (release, released) = mpsc::channel::<()>();
    let (report, reported) = mpsc::channel();

    // `future`, `cx` and `waker` drop at the end of this block while the work still waits on
    // the channel: past it, only the thread could still hold the waker the future left.
    {
        let waker = Waker::from(probe.clone());
        let mut cx = Context::from_waker(&waker);
        let mut future = unblock(move || {
            released.recv().expect("the test lets the work go on");
            report.send(()).expect("the test waits for the work");
        });

        assert!(Pin::new(&mut future).poll(&mut cx).is_pending());
    }

    assert_eq!(
        Arc::strong_count(&probe),
        1,
        "the thread kept the waker of a future that was dropped",
    );
    release.send(()).expect("the work waits for the test");
    reported.recv().expect("the work runs to its end");
}

/// The thread wakes the waker it was left, and so lets go of it, before the outcome can be seen:
/// by the time the future resolves, nothing but the test's own handle refers to it.
#[test]
#[timeout(15000)]
fn the_thread_drops_the_waker_before_the_future_it_wakes_can_resolve() {
    let probe = Arc::new(SlowWake);

    // `future`, `cx` and `waker` drop, in that order, at the end of this block: past it,
    // nothing but `probe` itself still refers to the waker the future was polled with.
    let value = {
        let waker = Waker::from(probe.clone());
        let mut cx = Context::from_waker(&waker);
        let mut future = unblock(|| {
            thread::sleep(Duration::from_millis(20));
            42
        });

        loop {
            match Pin::new(&mut future).poll(&mut cx) {
                Poll::Ready(value) => break value,
                Poll::Pending => thread::sleep(Duration::from_millis(1)),
            }
        }
    };

    assert_eq!(value, 42);
    assert_eq!(
        Arc::strong_count(&probe),
        1,
        "the thread's waker outlived the future it woke",
    );
}

/// A waker that unparks the thread it was made on, as a `block_on` does.
struct Unpark(thread::Thread);

impl Wake for Unpark {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}

/// A waker whose `wake` takes long enough that a poll able to take the lock as soon as this
/// thread lets go of the waker, rather than only once it is done with it, would resolve the
/// future well before this returns.
struct SlowWake;

impl Wake for SlowWake {
    fn wake(self: Arc<Self>) {
        thread::sleep(Duration::from_millis(50));
    }
}
