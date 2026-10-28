//! Tests of the `helper` feature: the helper thread that starts on the first piece of work
//! handed to a runtime with a seat from outside `block_on`, the `block_on` that runs the scheduler
//! and the reactor on its own thread, the hand-over between the two, and the registries
//! [`SharedRuntime::current`] hands runtimes out of.

use std::{
    cell::RefCell,
    future::{Future, Pending, pending, poll_fn},
    io::{self, Read, Write},
    mem::MaybeUninit,
    pin::{Pin, pin},
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc,
    },
    task::{Context, Poll, Wake, Waker},
    thread,
    time::{Duration, Instant},
};

use futures::{executor::block_on, future::poll_immediate};
use ntest::timeout;
use socket2::SockRef;

use super::{
    async_io::{BUDGET, READS, note_progress, tcp_pair_holding},
    core::pair,
    yield_now,
};
use crate::{
    AsyncIo, Interest, Interval, LocalRuntime, Registration, Shared, SharedRuntime, Sleep, Task,
    Timeout, driver,
    mode::sealed::Sealed as _,
    runtime::{Core, lock},
};
#[cfg(feature = "event")]
use crate::{Event, EventListener};

/// What a handle is shared as: a task handle and a registration live in state that several threads
/// reach, a runtime handle is cloned into the tasks it spawns, and a timer, a timeout or an
/// interval goes wherever the task awaiting it is polled.
#[test]
#[timeout(15000)]
fn handles_cross_threads() {
    fn shared<T>()
    where
        T: Send + Sync + Unpin,
    {
    }
    fn sent<T>()
    where
        T: Send,
    {
    }

    shared::<SharedRuntime>();
    shared::<Task<(), Shared>>();
    shared::<Registration<Shared>>();
    sent::<Sleep<Shared>>();
    sent::<Timeout<Pending<()>, Shared>>();
    sent::<Interval<Shared>>();
}

#[test]
#[timeout(15000)]
fn a_spawned_task_runs_to_completion() {
    let runtime = runtime();
    let ran = Arc::new(AtomicBool::new(false));
    let task = {
        let ran = ran.clone();
        runtime.spawn("a task that sets a flag", async move {
            ran.store(true, Ordering::Release);
        })
    };

    block_on(task).unwrap();

    assert!(ran.load(Ordering::Acquire));
}

#[test]
#[timeout(15000)]
fn a_spawned_task_hands_its_output_back() {
    let runtime = runtime();
    let task = runtime.spawn("an answer", async { 42 });

    assert_eq!(block_on(task).unwrap(), 42);
}

#[test]
#[timeout(15000)]
fn a_finished_task_whose_handle_is_dropped_leaves_nothing_behind() {
    let runtime = runtime();
    let task = runtime.spawn("a task that finishes at once", async {});
    // Run the task to completion on this thread, keeping the handle alive past it.
    drive(&runtime, async {
        std::future::poll_fn(|cx| {
            cx.waker().wake_by_ref();
            std::task::Poll::<()>::Ready(())
        })
        .await
    });
    assert!(within_a_second(|| !runtime.inner().is_busy()));

    drop(task);

    assert!(!runtime.inner().is_busy());
    // A helper that gave the seat up to the call above is parked until that call leaves and
    // rouses it, so the round it retires in comes a moment later.
    assert!(helper_gone(&runtime));
}

#[test]
#[timeout(15000)]
fn the_helper_starts_on_the_first_spawn() {
    let runtime = runtime();
    let task = runtime.spawn("a task that names the thread it runs on", async {
        thread::current().name().map(str::to_owned)
    });

    let name = block_on(task).unwrap();

    assert_eq!(name.as_deref(), Some("zruntime"));
}

#[test]
#[timeout(15000)]
fn the_helper_exits_when_nothing_is_left() {
    let runtime = runtime();
    // A timer for the task to be, so that the deadline is armed by the helper's poll of it
    // rather than from this thread.
    let sleep = runtime.sleep(Duration::from_millis(50));
    let task = runtime.spawn("a task that sleeps", sleep);

    block_on(task).unwrap();

    assert!(helper_gone(&runtime));
}

#[test]
#[timeout(15000)]
fn cancelling_the_last_task_from_another_thread_lets_the_helper_exit() {
    let runtime = runtime();
    let task = runtime.spawn("a task that never finishes", pending::<()>());
    assert!(runtime.helper_running());

    drop(task);

    assert!(helper_gone(&runtime));
}

/// A task cancelled from another thread, through `cancel`, lets the helper exit once it was the
/// last, as a task dropped there does.
#[test]
#[timeout(15000)]
fn cancelling_the_last_task_from_another_thread_and_waiting_lets_the_helper_exit() {
    let runtime = runtime();
    let task = runtime.spawn("a task that never finishes", pending::<u32>());
    assert!(runtime.helper_running());

    assert_eq!(block_on(task.cancel()), None);

    assert!(helper_gone(&runtime));
}

/// A task cancelled while the helper thread polls it is dropped by the helper once that poll
/// returns, and the wait for it, on another thread, lasts until then.
#[test]
#[timeout(15000)]
fn cancelling_a_task_the_helper_polls_waits_for_that_poll() {
    let runtime = runtime();
    // The task's future holds a clone, so that the count tells whether that future is gone.
    let held = Arc::new(());
    let (announce, announced) = mpsc::channel();
    let (release, released) = mpsc::channel::<()>();
    let task = {
        let held = held.clone();
        runtime.spawn("a task held in its poll", async move {
            let _held = held;
            announce.send(()).unwrap();
            // A sender dropped by a test that failed first lets the poll go on as well.
            let _ = released.recv();
            pending::<u32>().await
        })
    };
    // The helper is inside the task's first poll once this is in.
    announced.recv().unwrap();

    let mut cancelling = pin!(task.cancel());

    assert!(block_on(poll_immediate(cancelling.as_mut())).is_none());
    assert_eq!(Arc::strong_count(&held), 2);
    release.send(()).unwrap();
    assert_eq!(block_on(cancelling), None);
    assert_eq!(Arc::strong_count(&held), 1);
    assert!(helper_gone(&runtime));
}

/// A wake that crosses from the thread in one runtime's seat to a task on another reaches it.
///
/// Two runtimes can be alive in one process, each with tasks of its own, so a task of one can
/// wake a task of the other from the thread that holds the first runtime's seat. The thread in
/// the second runtime's seat is asleep in a wait of its own by then and has to be told over its
/// wake channel; a wake is only ever left unsent when it comes from the very thread it would be
/// telling.
///
/// The waiting task announces itself and the thread here waits that announcement out, so that
/// the wake is sent to a runtime whose thread has nothing of its own left to poll and is
/// therefore in a wait that only its wake channel can end.
#[cfg(feature = "event")]
#[test]
#[timeout(15000)]
fn a_task_on_one_runtime_wakes_a_task_on_another() {
    let waiting_runtime = runtime();
    let waking_runtime = runtime();
    let listening = Arc::new(Event::new());
    let woken = Arc::new(Event::new());

    // Taken before the task is spawned, so the announcement cannot be missed.
    let announced = listening.listen();
    let waiting = {
        let woken = woken.clone();
        waiting_runtime.spawn("a task waiting to be woken", async move {
            let wake = woken.listen();
            listening.notify(1);
            wake.await;

            42
        })
    };
    block_on(announced);
    // The announcement is made inside the poll that leaves the task waiting, so the helper
    // reaches its wait a moment after it; this is that moment.
    thread::sleep(Duration::from_millis(100));

    let _waking = waking_runtime.spawn("a task on another runtime", async move {
        woken.notify(1);
    });

    assert_eq!(block_on(waiting).unwrap(), 42);
}

#[test]
#[timeout(15000)]
fn a_finished_detached_task_lets_the_helper_exit() {
    let runtime = runtime();
    // A timer to wait out first, so that the round the task ends in is one the helper reached
    // from a wait that its own timer broke: the wake channel is empty by then, and nobody is
    // told the task is over, since nobody holds a handle to it.
    let sleep = runtime.sleep(Duration::from_millis(50));

    runtime.spawn("a detached task", sleep).detach();

    // Which leaves the helper to work its own idleness out.
    assert!(helper_gone(&runtime));
}

#[test]
#[timeout(15000)]
fn a_registration_reports_readiness() {
    let runtime = runtime();
    let (source, mut peer) = pair();
    let registration = runtime.register(source.clone()).unwrap();
    // Written from another thread once the read below is waiting, so that the byte is one the
    // helper's wait reports rather than one the first attempt to read finds for itself.
    let writer = thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        peer.write_all(&[7]).unwrap();
    });

    let read = block_on(poll_fn(|cx| {
        let mut byte = [MaybeUninit::<u8>::uninit(); 1];

        registration.poll_io(cx, Interest::Readable, || {
            SockRef::from(&source).recv(&mut byte)
        })
    }));

    assert_eq!(read.unwrap(), 1);
    writer.join().unwrap();
}

/// A socket of one protocol family is watched beside a wake channel of another.
///
/// Winsock takes every socket of one `select` call to come from a single service provider, and a
/// runtime's wake channel is a loopback TCP connection while a caller may register an `AF_UNIX`
/// socket directly. A wait therefore holds sockets of both families, and this is where that
/// mixture is asked for: an `AF_UNIX` socket registered on a runtime is reported readable just as
/// a socket of the wake channel's own family is.
#[cfg(windows)]
#[test]
#[timeout(15000)]
fn an_af_unix_socket_is_watched_beside_the_tcp_wake_socket() {
    use std::{
        env, fs,
        os::windows::io::{AsSocket, OwnedSocket},
        process,
        time::SystemTime,
    };

    use uds_windows::{UnixListener, UnixStream};

    let runtime = runtime();
    // Named after this process and this moment, so that two runs of the suite cannot meet over
    // one path.
    let since_epoch = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let path = env::temp_dir().join(format!(
        "zruntime-{}-{}.sock",
        process::id(),
        since_epoch.as_nanos()
    ));
    let listener = UnixListener::bind(&path).unwrap();
    let client = UnixStream::connect(&path).unwrap();
    let (mut server, _) = listener.accept().unwrap();
    client.set_nonblocking(true).unwrap();
    server.set_nonblocking(true).unwrap();
    // The stream keeps the handle it was made with, so what the source is given is a duplicate
    // of it.
    let owned: OwnedSocket = client.as_socket().try_clone_to_owned().unwrap();
    let source: Arc<OwnedSocket> = Arc::new(owned);
    let registration = runtime.register(source.clone()).unwrap();
    // Written from another thread once the read below is waiting, so that the byte is one the
    // helper's wait reports rather than one the first attempt to read finds for itself.
    let writer = thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        server.write_all(&[7]).unwrap();
    });

    let read = block_on(poll_fn(|cx| {
        let mut byte = [MaybeUninit::<u8>::uninit(); 1];

        registration.poll_io(cx, Interest::Readable, || {
            SockRef::from(&source).recv(&mut byte)
        })
    }));

    assert_eq!(read.unwrap(), 1);
    writer.join().unwrap();
    drop(listener);
    fs::remove_file(&path).unwrap();
}

#[test]
#[timeout(15000)]
fn sleep_resolves_once_the_duration_has_passed() {
    let runtime = runtime();
    let started = Instant::now();

    block_on(runtime.sleep(Duration::from_millis(50)));

    assert!(started.elapsed() >= Duration::from_millis(50));
}

#[test]
#[timeout(15000)]
fn a_sleep_polled_after_the_helper_retired_is_fired() {
    let runtime = runtime();
    let sleep = runtime.sleep(Duration::from_millis(20));
    // A timer that has never been polled has no deadline with the reactor, so there is nothing
    // for a helper to do and none of them runs.
    assert!(!runtime.helper_running());
    let started = Instant::now();

    block_on(sleep);

    assert!(started.elapsed() >= Duration::from_millis(20));
    assert!(started.elapsed() < Duration::from_secs(1));
}

#[test]
#[timeout(15000)]
fn a_sleep_armed_from_another_thread_bounds_the_wait() {
    let runtime = runtime();
    let (source, _peer) = pair();
    // A source to watch and no deadline: the helper sits in a wait that nothing but a
    // notification brings to an end.
    let _registration = runtime.register(source).unwrap();
    let started = Instant::now();

    block_on(runtime.sleep(Duration::from_millis(20)));

    assert!(started.elapsed() < Duration::from_millis(500));
}

#[test]
#[timeout(15000)]
fn a_dropped_sleep_lets_the_helper_exit() {
    let runtime = runtime();
    {
        let mut sleep = pin!(runtime.sleep(Duration::from_secs(10)));
        // The first poll is what hands the deadline to the reactor.
        assert!(block_on(poll_immediate(sleep.as_mut())).is_none());
        assert!(runtime.helper_running());
    }

    // The deadline went with the timer it belonged to, so the wait it bounded is over too.
    assert!(helper_gone(&runtime));
}

/// A timer made for a deadline asks for the helper on its first poll, as one made for a duration
/// does, and lets it go once dropped.
#[test]
#[timeout(15000)]
fn a_sleep_until_starts_the_helper_and_dropping_it_lets_it_exit() {
    let runtime = runtime();
    {
        let mut sleep = pin!(runtime.sleep_until(Instant::now() + Duration::from_secs(10)));
        // Made but not yet polled, the timer has handed the reactor nothing to wait on.
        assert!(!runtime.helper_running());
        assert!(block_on(poll_immediate(sleep.as_mut())).is_none());
        assert!(runtime.helper_running());
    }

    assert!(helper_gone(&runtime));
}

/// A timer the helper waits on is fired at the deadline it is reset to, earlier than the one it
/// was polled with, though nothing polls it again.
#[test]
#[timeout(15000)]
fn the_helper_fires_a_sleep_reset_to_an_earlier_deadline() {
    let runtime = runtime();
    let woken = Arc::new(Woken::default());
    let waker = Waker::from(woken.clone());
    let mut sleep = runtime.sleep(Duration::from_secs(10));
    // Polled outside any `block_on`, so that the thread waiting on the deadline is the helper.
    let polled = Pin::new(&mut sleep).poll(&mut Context::from_waker(&waker));
    assert!(polled.is_pending());
    assert!(runtime.helper_running());
    // Long enough for the helper to reach its wait, bounded by the deadline ten seconds ahead,
    // which the reset then has to break.
    thread::sleep(Duration::from_millis(50));

    sleep.reset_after(Duration::from_millis(20));

    let deadline = Instant::now() + Duration::from_secs(5);
    while woken.count() == 0 && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(woken.count(), 1);
}

/// A timer reset to never come due gives up the deadline that kept the helper waiting, and the
/// helper, with nothing left to wait on, exits while the timer itself lives on.
#[test]
#[timeout(15000)]
fn a_sleep_reset_beyond_the_clock_lets_the_helper_exit() {
    let runtime = runtime();
    let mut sleep = pin!(runtime.sleep(Duration::from_secs(10)));
    assert!(block_on(poll_immediate(sleep.as_mut())).is_none());
    assert!(runtime.helper_running());
    // Long enough for the helper to reach its wait, bounded by the deadline ten seconds ahead,
    // which the reset then has to break.
    thread::sleep(Duration::from_millis(50));

    sleep.reset_after(Duration::MAX);

    assert!(helper_gone(&runtime));
    assert!(!runtime.inner().is_busy());
}

/// A timeout whose future is left waiting asks for the helper, as a timer does on its first poll,
/// and lets it go once dropped.
#[test]
#[timeout(15000)]
fn a_timeout_starts_the_helper_and_dropping_it_lets_it_exit() {
    let runtime = runtime();
    {
        let mut timeout = pin!(runtime.timeout(Duration::from_secs(10), pending::<()>()));
        // Made but not yet polled, the timeout has handed the reactor nothing to wait on.
        assert!(!runtime.helper_running());
        assert!(block_on(poll_immediate(timeout.as_mut())).is_none());
        assert!(runtime.helper_running());
    }

    assert!(helper_gone(&runtime));
}

/// A timeout whose future completes on its first poll never polls its timer, so no deadline
/// reaches the reactor and no helper is asked for.
#[test]
#[timeout(15000)]
fn a_timeout_whose_future_completes_at_once_starts_no_helper() {
    let runtime = runtime();

    let output = block_on(runtime.timeout(Duration::from_secs(10), async { 7 }));

    assert_eq!(output.ok(), Some(7));
    assert!(!runtime.helper_running());
    assert!(!runtime.inner().is_busy());
}

/// An interval asks for the helper on its first poll, as a timer does, and lets it go once
/// dropped.
#[test]
#[timeout(15000)]
fn an_interval_starts_the_helper_and_dropping_it_lets_it_exit() {
    let runtime = runtime();
    {
        let mut interval = runtime.interval(Duration::from_secs(10));
        // Made but not yet polled, the interval has handed the reactor nothing to wait on.
        assert!(!runtime.helper_running());
        assert!(block_on(poll_immediate(interval.tick())).is_none());
        assert!(runtime.helper_running());
    }

    assert!(helper_gone(&runtime));
}

/// An interval nobody drives through `block_on` keeps ticking on the helper alone: each tick's
/// wake comes from the helper, and the poll after it, which moves the interval on to its next
/// tick, asks for the helper again where it has retired in between.
#[test]
#[timeout(15000)]
fn the_helper_keeps_an_interval_ticking() {
    let runtime = runtime();
    let period = Duration::from_millis(20);
    let woken = Arc::new(Woken::default());
    let waker = Waker::from(woken.clone());
    let mut cx = Context::from_waker(&waker);
    let mut interval = runtime.interval(period);
    let mut ticks = Vec::new();

    while ticks.len() < 3 {
        let seen = woken.count();
        match interval.poll_tick(&mut cx) {
            Poll::Ready(tick) => ticks.push(tick),
            Poll::Pending => {
                let deadline = Instant::now() + Duration::from_secs(5);
                while woken.count() == seen && Instant::now() < deadline {
                    thread::sleep(Duration::from_millis(5));
                }
                assert!(woken.count() > seen);
            }
        }
    }

    assert_eq!(ticks[1] - ticks[0], period);
    assert_eq!(ticks[2] - ticks[1], period);
}

#[test]
#[timeout(15000)]
fn a_sleep_beyond_the_clock_starts_no_helper() {
    let runtime = runtime();
    let mut sleep = pin!(runtime.sleep(Duration::MAX));

    assert!(block_on(poll_immediate(sleep.as_mut())).is_none());

    // A timer nothing can ever fire has no deadline for a thread to wait on, so none is asked
    // for: a helper started here would retire in the very round it started.
    assert!(!runtime.helper_running());
    assert!(!runtime.inner().is_busy());
}

#[test]
#[timeout(15000)]
fn an_interval_beyond_the_clock_starts_no_helper() {
    let runtime = runtime();
    let mut interval = runtime.interval(Duration::MAX);

    assert!(block_on(poll_immediate(interval.tick())).is_none());

    // An interval whose first tick the clock cannot name has no deadline for a thread to wait
    // on, as a timer that is never to fire has none, so no helper is asked for.
    assert!(!runtime.helper_running());
    assert!(!runtime.inner().is_busy());
}

#[test]
#[timeout(15000)]
fn a_spawn_during_the_helper_exit_is_not_stranded() {
    let runtime = runtime();

    // No pause between the rounds, so that each spawn lands wherever the round before it left
    // the helper, the exit it is about to make included.
    for _ in 0..200 {
        let started = Instant::now();
        let task = runtime.spawn("a task that finishes at once", async {});

        block_on(task).unwrap();

        assert!(started.elapsed() < Duration::from_secs(5));
    }
}

#[test]
#[timeout(15000)]
fn a_registration_without_tasks_keeps_the_helper_alive() {
    let runtime = runtime();
    let (source, _peer) = pair();

    let _registration = runtime.register(source).unwrap();

    assert!(runtime.helper_running());
    // Long enough that a helper taking no account of the registration would have retired.
    thread::sleep(Duration::from_millis(200));
    assert!(runtime.helper_running());
}

#[test]
#[timeout(15000)]
fn a_task_that_panics_leaves_the_helper_running() {
    let runtime = runtime();
    // Keeps the helper from retiring between the tasks below, so that the task after the panic
    // is run by the very helper the panic happened on.
    let _held = runtime.spawn("a task that never finishes", pending::<()>());
    let panicking = runtime.spawn("a task that panics", async {
        panic!("the task panicked on purpose");
    });

    assert!(block_on(panicking).is_err());

    // The task that never finishes rules retirement out, so a helper running here is the one the
    // panic happened on rather than one a later spawn started.
    assert!(runtime.helper_running());
    let next = runtime.spawn("the task after it", async { 7 });
    assert_eq!(block_on(next).unwrap(), 7);
}

#[test]
#[timeout(15000)]
fn a_panic_in_a_futures_drop_leaves_the_helper_running() {
    /// A task that is finished with on its first poll, and whose future panics as the helper
    /// drops it.
    struct PanicOnDrop;

    impl Future for PanicOnDrop {
        type Output = ();

        fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
            Poll::Ready(())
        }
    }

    impl Drop for PanicOnDrop {
        fn drop(&mut self) {
            panic!("the future's drop panicked on purpose");
        }
    }

    let runtime = runtime();
    let _held = runtime.spawn("a task that never finishes", pending::<()>());
    let going = runtime.spawn("a task that panics as it goes", PanicOnDrop);

    assert!(block_on(going).is_ok());

    // The task that never finishes rules retirement out, so a helper running here is the one
    // that dropped the future above rather than one a later spawn started.
    assert!(runtime.helper_running());
    let next = runtime.spawn("the task after it", async { 7 });
    assert_eq!(block_on(next).unwrap(), 7);
}

#[test]
#[timeout(15000)]
fn a_cancelled_futures_drop_may_spawn_on_the_runtime() {
    /// Spawns a task of its own as it goes, through the runtime it holds.
    struct SpawnOnDrop {
        runtime: SharedRuntime,
        ran: Arc<AtomicBool>,
    }

    impl Drop for SpawnOnDrop {
        fn drop(&mut self) {
            let ran = self.ran.clone();
            self.runtime
                .spawn("a task spawned from a drop", async move {
                    ran.store(true, Ordering::Release);
                })
                .detach();
        }
    }

    let runtime = runtime();
    let ran = Arc::new(AtomicBool::new(false));
    let polled = Arc::new(AtomicBool::new(false));
    let task = {
        let spawner = SpawnOnDrop {
            runtime: runtime.clone(),
            ran: ran.clone(),
        };
        let polled = polled.clone();
        runtime.spawn("a task that never finishes", async move {
            let _spawner = spawner;
            polled.store(true, Ordering::Release);
            pending::<()>().await;
        })
    };

    // Cancelled once the helper has taken the task up, so that the drop of its future is this
    // thread's to make. A cancellation racing that poll would leave the drop to the helper
    // instead, and the spawn the drop makes is served the same either way.
    assert!(within_a_second(|| polled.load(Ordering::Acquire)));
    drop(task);

    assert!(within_a_second(|| ran.load(Ordering::Acquire)));
}

#[test]
#[timeout(15000)]
fn a_helper_that_dies_is_replaced() {
    /// Panics where it is woken, which is on the helper that fires the timer it waits for.
    struct PanickingWaker;

    impl Wake for PanickingWaker {
        fn wake(self: Arc<Self>) {
            panic!("the waker panicked on purpose");
        }
    }

    let runtime = runtime();
    // A task that never finishes, so that retiring is out of the question and an unwind is the
    // one thing that can clear the running flag.
    let _held = runtime.spawn("a task that never finishes", pending::<()>());
    let waker = Waker::from(Arc::new(PanickingWaker));
    let mut sleep = pin!(runtime.sleep(Duration::from_millis(1)));
    let polled = sleep.as_mut().poll(&mut Context::from_waker(&waker));
    assert!(polled.is_pending());

    // The helper fires the timer, the wake panics, and the thread unwinds out of its loop.
    assert!(helper_gone(&runtime));

    let task = runtime.spawn("the task after it", async { 7 });
    assert_eq!(block_on(task).unwrap(), 7);
}

#[test]
#[timeout(15000)]
fn block_on_runs_the_tasks_on_its_own_thread() {
    let runtime = runtime();

    // Spawned from inside, so that the runtime is handed its first piece of work by the thread
    // in the seat, which asks for no helper and runs that work itself.
    let ran_on = drive(&runtime, async {
        runtime
            .spawn("a task that names the thread it runs on", async {
                thread::current().id()
            })
            .await
    })
    .unwrap();

    assert_eq!(ran_on, thread::current().id());
    assert!(!runtime.helper_running());
}

#[test]
#[timeout(15000)]
fn block_on_fires_a_timer_on_its_own_thread() {
    let runtime = runtime();
    let started = Instant::now();

    drive(&runtime, runtime.sleep(Duration::from_millis(20)));

    assert!(started.elapsed() >= Duration::from_millis(20));
    assert!(!runtime.helper_running());
}

#[test]
#[timeout(15000)]
fn block_on_waits_for_readiness_on_its_own_thread() {
    let runtime = runtime();
    let (source, mut peer) = pair();
    // Written from another thread once the read below is waiting, so that the byte is one the
    // wait reports rather than one the first attempt to read finds for itself.
    let writer = thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        peer.write_all(&[7]).unwrap();
    });

    let read = drive(&runtime, async {
        // Registered from inside, so that the runtime is handed its first piece of work by the
        // thread in the seat, which asks for no helper and does the waiting itself. The
        // registration goes with this future, and so is gone before the seat is given up.
        let registration = runtime.register(source.clone()).unwrap();

        poll_fn(|cx| {
            let mut byte = [MaybeUninit::<u8>::uninit(); 1];

            registration.poll_io(cx, Interest::Readable, || {
                SockRef::from(&source).recv(&mut byte)
            })
        })
        .await
    });

    assert_eq!(read.unwrap(), 1);
    assert!(!runtime.helper_running());
    writer.join().unwrap();
}

/// The future of a `block_on` that sits in the seat has a budget of I/O operations as a task has:
/// a task spawned from it runs after its budget's worth of reads, though the future itself would
/// read on to the end. It is the tasks that the thread in the seat runs between the polls of the
/// future, so a future that does not return would hold them all up.
#[test]
#[timeout(15000)]
fn the_future_of_block_on_yields_to_the_tasks_after_the_budget() {
    let runtime = runtime();
    let (near, _far) = tcp_pair_holding(READS);
    let (reads, seen) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));

    drive(&runtime, async {
        // Registered and spawned from inside, so that the thread in the seat runs the task itself
        // rather than a helper that the first piece of work from outside would start.
        let near = AsyncIo::new(&runtime, near).unwrap();
        let looker = runtime.spawn("a looker", note_progress(reads.clone(), seen.clone()));
        for _ in 0..READS {
            near.read_with(|mut stream| stream.read(&mut [0]))
                .await
                .unwrap();
            reads.fetch_add(1, Ordering::SeqCst);
        }
        looker.await.unwrap();
    });

    assert_eq!(seen.load(Ordering::SeqCst), BUDGET);
    assert_eq!(reads.load(Ordering::SeqCst), READS);
    assert!(!runtime.helper_running());
}

#[test]
#[timeout(15000)]
fn a_wake_from_another_thread_ends_the_wait_block_on_is_in() {
    let runtime = runtime();
    // Nothing to watch and no deadline, so the wait below is bounded by nothing but a
    // notification.
    let wakers = Arc::new(Mutex::new(None::<Waker>));
    let waker_thread = {
        let wakers = wakers.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));
            if let Some(waker) = lock(&wakers).take() {
                waker.wake();
            }
        })
    };
    let started = Instant::now();

    drive(&runtime, async {
        let mut polled = false;
        poll_fn(|cx| {
            if polled {
                return Poll::Ready(());
            }
            polled = true;
            *lock(&wakers) = Some(cx.waker().clone());

            Poll::Pending
        })
        .await
    });

    assert!(started.elapsed() >= Duration::from_millis(50));
    assert!(started.elapsed() < Duration::from_secs(5));
    waker_thread.join().unwrap();
}

#[test]
#[timeout(15000)]
fn work_left_behind_by_block_on_goes_to_a_helper() {
    let runtime = runtime();
    let task = runtime.spawn("a task that never finishes", pending::<()>());
    // Spawned from outside `block_on` while nobody drives, so a helper starts here already; it
    // is the one that has to be there once `block_on` has returned that is tested, so the
    // spawn is made from inside.
    drop(task);
    assert!(helper_gone(&runtime));

    // Kept in a place of its own rather than handed out as the future's output: a task handle is
    // a future itself, and an async block that yields one reads as though it were to be awaited.
    let mut task = None;
    drive(&runtime, async {
        task = Some(runtime.spawn("a task that never finishes", pending::<()>()));
    });

    assert!(within_a_second(|| runtime.helper_running()));
    drop(task);
    assert!(helper_gone(&runtime));
}

#[test]
#[timeout(15000)]
fn work_left_by_a_block_on_that_took_no_seat_goes_to_a_helper() {
    let runtime = runtime();
    let (resolve, _own) = resolve_on_the_second_ask(&runtime);
    let mut task = None;

    driver::block_on(driver::Target::Own, &*resolve, async {
        task = Some(runtime.spawn("a task that never finishes", pending::<()>()));
    });

    assert!(within_a_second(|| runtime.helper_running()));
    drop(task);
    assert!(helper_gone(&runtime));
}

/// A panic out of a seat-less `block_on` hands the work on all the same.
///
/// The future that panics may have spawned a task which outlives it, stashed somewhere the
/// panic does not reach. That task was spawned while this thread stood surety for running it,
/// and has nobody to run it once the thread has unwound.
#[test]
#[timeout(15000)]
fn work_left_by_a_block_on_that_panicked_without_the_seat_goes_to_a_helper() {
    let runtime = runtime();
    let (resolve, _own) = resolve_on_the_second_ask(&runtime);
    let mut task = None;

    let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        driver::block_on(driver::Target::Own, &*resolve, async {
            task = Some(runtime.spawn("a task that never finishes", pending::<()>()));
            panic!("a future that panics once it has spawned");
        });
    }));
    assert!(panicked.is_err());

    assert!(within_a_second(|| runtime.helper_running()));
    drop(task);
    assert!(helper_gone(&runtime));
}

#[cfg(feature = "event")]
#[test]
#[timeout(15000)]
fn a_block_on_takes_the_seat_from_the_helper() {
    let runtime = runtime();
    let _held = held_by_the_helper(&runtime);
    let (parked, watcher) = the_parked_helper_announced(&runtime);

    let ran_on = drive(&runtime, async {
        // Awaited before the spawn below, so that the spawn is made by a thread that has the
        // seat: the helper gives it up as it finds this call waiting for it, and stays parked
        // until the call leaves.
        parked.await;
        runtime
            .spawn("a task that names the thread it runs on", async {
                thread::current().id()
            })
            .await
    })
    .unwrap();

    assert!(watcher.join().unwrap());
    assert_eq!(ran_on, thread::current().id());
}

#[cfg(feature = "event")]
#[test]
#[timeout(15000)]
fn the_helper_takes_the_seat_back_when_block_on_leaves() {
    let runtime = runtime();
    let _held = held_by_the_helper(&runtime);
    let (parked, watcher) = the_parked_helper_announced(&runtime);
    // Asks the helper for the seat and leaves once it has it, so that what is tested below is
    // the helper taking the seat back rather than never having given it up.
    drive(&runtime, parked);
    assert!(watcher.join().unwrap());

    // Awaited from outside `block_on`, so that the helper is the one thing that can run it.
    let task = runtime.spawn("a task that names the thread it runs on", async {
        thread::current().name().map(str::to_owned)
    });
    let name = block_on(task).unwrap();

    assert_eq!(name.as_deref(), Some("zruntime"));
}

/// A call that never takes the seat up rouses the helper it parked all the same.
///
/// A `block_on` asks for the seat before it polls, and the helper gives it up in the round the
/// asking wakes it for. A future that is done at that first poll leaves the seat free with the
/// helper parked, and nothing but this call is there to tell the helper to look again.
#[cfg(feature = "event")]
#[test]
#[timeout(15000)]
fn a_block_on_that_leaves_without_the_seat_rouses_the_helper() {
    let runtime = runtime();
    let held = held_by_the_helper(&runtime);
    let (parked, watcher) = the_parked_helper_announced(&runtime);

    drive(&runtime, async {
        // Waited out inside the poll rather than awaited, so that the call leaves from this
        // very poll: a future that went back to its caller pending would be handed the seat
        // the helper freed, and this call would be the one to give it up again.
        block_on(parked);
    });

    assert!(watcher.join().unwrap());
    // Roused by the call above as it left, the helper is back in the seat.
    assert!(within_a_second(|| !runtime.helper_parked()));
    drop(held);
    assert!(helper_gone(&runtime));
}

#[cfg(feature = "event")]
#[test]
#[timeout(15000)]
fn a_parked_helper_leaves_once_nothing_is_left() {
    let runtime = runtime();
    let held = held_by_the_helper(&runtime);
    // Asks the helper for the seat, which leaves the helper parked or back in it, and up
    // either way: the task above rules retiring out.
    drive(&runtime, async {});
    assert!(runtime.helper_running());

    drop(held);

    assert!(helper_gone(&runtime));
}

#[cfg(feature = "event")]
#[test]
#[timeout(15000)]
fn a_second_block_on_parks_while_the_first_drives() {
    let runtime = runtime();
    let release = Arc::new(Event::new());
    let driver = {
        let runtime = runtime.clone();
        let release = release.clone();
        thread::spawn(move || {
            drive(&runtime, async move {
                release.listen().await;
            })
        })
    };
    // Long enough for the thread above to take the seat.
    thread::sleep(Duration::from_millis(50));
    let task = runtime.spawn("a task that names the thread it runs on", async {
        thread::current().id()
    });

    let ran_on = drive(&runtime, task).unwrap();

    assert_eq!(ran_on, driver.thread().id());
    assert!(!runtime.helper_running());
    release.notify(1);
    driver.join().unwrap();
}

/// A `block_on` that waited for the seat leaves nothing of itself on the runtime.
///
/// A thread-per-request program calling `block_on`, where a helper keeps the seat for a task's
/// whole life, would gather one such handle per thread it ever ran and hold them for the life of
/// the process, were the entry not taken back where its call returns.
#[cfg(feature = "event")]
#[test]
#[timeout(15000)]
fn a_block_on_that_waited_for_the_seat_leaves_nothing_behind() {
    let runtime = runtime();
    let release = Arc::new(Event::new());
    let driver = {
        let runtime = runtime.clone();
        let release = release.clone();
        thread::spawn(move || {
            drive(&runtime, async move {
                release.listen().await;
            })
        })
    };
    // Long enough for the thread above to take the seat, which it keeps until it is released.
    thread::sleep(Duration::from_millis(50));
    let task = runtime.spawn("a task that names the thread it runs on", async {
        thread::current().id()
    });

    let ran_on = drive(&runtime, task).unwrap();

    // The task ran on the other thread, which says the seat was that thread's throughout and
    // this one waited for it rather than took it.
    assert_eq!(ran_on, driver.thread().id());
    assert_eq!(waiting(&runtime), 0);
    release.notify(1);
    driver.join().unwrap();
}

#[test]
#[timeout(15000)]
fn a_block_on_inside_a_task_panics_rather_than_hangs() {
    let runtime = runtime();
    let task = runtime.spawn("a task that calls block_on", {
        let runtime = runtime.clone();
        async move {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                drive(&runtime, async {});
            }))
            .is_err()
        }
    });

    assert!(drive(&runtime, task).unwrap());
}

#[test]
#[timeout(15000)]
fn a_panic_in_the_future_frees_the_seat() {
    let runtime = runtime();
    let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        drive(&runtime, async { panic!("a future that panics") });
    }));
    assert!(panicked.is_err());

    let task = runtime.spawn("the task after it", async { 7 });
    assert_eq!(drive(&runtime, task).unwrap(), 7);
}

#[test]
#[timeout(15000)]
fn every_handle_in_a_registry_shares_one_runtime() {
    let registry = Mutex::new(Weak::new());
    let first = SharedRuntime::from_inner(driver::shared_in(&registry).unwrap());
    let second = SharedRuntime::from_inner(driver::shared_in(&registry).unwrap());

    assert!(Arc::ptr_eq(first.inner(), second.inner()));
}

#[test]
#[timeout(15000)]
fn a_shared_runtime_goes_with_its_last_handle() {
    let registry = Mutex::new(Weak::new());
    let handle = SharedRuntime::from_inner(driver::shared_in(&registry).unwrap());
    let inner = Arc::downgrade(handle.inner());

    drop(handle);

    // Nothing was spawned or registered, so no thread holds the runtime either.
    assert!(inner.upgrade().is_none());
    // And the next handle brings a fresh one into being.
    let next = SharedRuntime::from_inner(driver::shared_in(&registry).unwrap());
    assert!(inner.upgrade().is_none());
    drop(next);
}

#[test]
#[timeout(15000)]
fn each_thread_inside_block_on_has_a_runtime_of_its_own() {
    // `SharedRuntime::current` on a thread that is in no seat goes to that thread's own runtime.
    // The two handles are held across the comparison, so that the second runtime cannot land on
    // the address the first has given up.
    let on_this_thread = crate::block_on(async { SharedRuntime::current().unwrap() });
    let on_another = thread::spawn(|| crate::block_on(async { SharedRuntime::current().unwrap() }))
        .join()
        .unwrap();

    assert!(!Arc::ptr_eq(on_this_thread.inner(), on_another.inner()));
    // And the same thread gets the same runtime back for as long as it is alive.
    let again = crate::block_on(async { SharedRuntime::current().unwrap() });
    assert!(Arc::ptr_eq(again.inner(), on_this_thread.inner()));
}

#[test]
#[timeout(15000)]
fn work_built_under_another_executor_shares_one_runtime() {
    // Built on threads that are inside no `block_on` and in no seat, as work some other executor
    // polls is: one runtime serves them all, and one helper runs it. The handles are held across
    // the comparison, so that the second cannot land on an address the first gave up.
    let here = SharedRuntime::current().unwrap();
    let there = thread::spawn(|| SharedRuntime::current().unwrap())
        .join()
        .unwrap();

    assert!(Arc::ptr_eq(here.inner(), there.inner()));
}

#[test]
#[timeout(15000)]
fn a_value_built_on_a_helper_thread_joins_the_runtime_the_helper_runs() {
    // A runtime in no registry, run by a helper because nobody is inside `block_on` on it.
    let runtime = runtime();
    // An address rather than a handle, a raw pointer being no more sendable than it is telling.
    let expected = Arc::as_ptr(runtime.inner()) as usize;

    let built_on = runtime.spawn("a task that builds a handle", async {
        Arc::as_ptr(SharedRuntime::current().unwrap().inner()) as usize
    });

    assert_eq!(block_on(built_on).unwrap(), expected);
}

#[test]
#[timeout(15000)]
fn a_spawn_from_inside_another_threads_block_on_starts_a_helper() {
    // A runtime in no registry, so that it is nobody's own; the thread below is inside
    // `block_on` with nothing of its own to drive.
    let runtime = runtime();
    let ran = Arc::new(AtomicBool::new(false));
    // Kept in a place of its own rather than handed out as the future's output: a task handle is
    // a future itself, and an async block that yields one reads as though it were to be awaited.
    let mut task = None;
    {
        let ran = ran.clone();
        driver::block_on(driver::Target::Own, &|| None, async {
            task = Some(
                runtime.spawn("a task of another thread's runtime", async move {
                    ran.store(true, Ordering::Release);
                }),
            );
        });
    }

    // Nobody but a helper can run it: the thread that spawned it drives no runtime of this one.
    block_on(task.unwrap()).unwrap();
    assert!(ran.load(Ordering::Acquire));
}

/// A value built as its thread ends is run rather than a panic.
///
/// `SharedRuntime::current` looks a thread's own runtime up in two thread-locals, and a task
/// spawned from the drop of another thread-local value — one stashed there, say — finds them gone.
/// A thread past running anything of its own is one no `block_on` is inside of, so what it gets is
/// the runtime the process shares, and a helper runs its work.
///
/// The value below is touched before the locals `SharedRuntime::current` reaches, so that its drop
/// runs after theirs: thread-local destructors run in reverse order of registration.
#[cfg(feature = "event")]
#[test]
#[timeout(15000)]
fn a_task_spawned_as_a_thread_ends_is_run_by_a_helper() {
    struct BuildsAsItGoes(Option<Arc<Event>>);

    impl Drop for BuildsAsItGoes {
        fn drop(&mut self) {
            let Some(ran) = self.0.take() else {
                return;
            };
            SharedRuntime::current()
                .unwrap()
                .spawn("a task spawned as a thread ends", async move {
                    ran.notify(1);
                })
                .detach();
        }
    }

    thread_local! {
        static BUILDS_AS_IT_GOES: RefCell<BuildsAsItGoes> =
            const { RefCell::new(BuildsAsItGoes(None)) };
    }

    let ran = Arc::new(Event::new());
    // Taken before the thread starts, so that the notification cannot be missed.
    let notified = ran.listen();
    let thread = thread::spawn(move || {
        BUILDS_AS_IT_GOES.with(|builds| builds.borrow_mut().0 = Some(ran));
        // Which is what brings the locals `SharedRuntime::current` reaches into being on this
        // thread.
        drop(SharedRuntime::current().unwrap());
    });

    assert!(thread.join().is_ok());
    block_on(notified);
}

/// A `block_on` as its thread ends runs rather than takes the process down with it.
///
/// A value awaited from the drop of a thread-local — `block_on(some_task).unwrap()`, say —
/// reaches the registry the call resolves in, and finds it gone. A panic there is not an unwind
/// but an abort, so the lookup has to come back empty: the call then takes no seat and polls and
/// parks, and whatever its future leaves behind goes to a helper.
///
/// The future also builds a handle, for the same lookup made from inside the call.
#[test]
#[timeout(15000)]
fn block_on_as_a_thread_ends_does_not_abort() {
    struct BlocksAsItGoes;

    impl Drop for BlocksAsItGoes {
        fn drop(&mut self) {
            crate::block_on(async { drop(SharedRuntime::current().unwrap()) });
        }
    }

    thread_local! {
        static BLOCKS_AS_IT_GOES: BlocksAsItGoes = const { BlocksAsItGoes };
    }

    let thread = thread::spawn(|| {
        BLOCKS_AS_IT_GOES.with(|_| ());
        // Which is what brings the locals a `block_on` reaches into being on this thread, the
        // thread's own registry among them: a thread that never had one has nothing to lose.
        crate::block_on(async { drop(SharedRuntime::current().unwrap()) });
    });

    assert!(thread.join().is_ok());
}

/// A `block_on` that takes the seat as its thread ends runs rather than aborts.
///
/// Thread-local destructors run in reverse order of registration, and a `block_on` registers the
/// locals it uses as it goes: the thread's own registry on the lookup it makes before its first
/// poll. A value the poll brings into being is therefore destroyed before the registry, and a
/// `block_on` from that value's destructor — waiting on a task stashed there, say — finds a live
/// runtime in the registry and takes the seat. The record of the seat it writes as it does is
/// first reached by the lookup made from inside that poll, after the value, so that a record with
/// a destructor of its own would be gone by then, leaving no place to write down that the thread
/// is in the seat: a panic there is not an unwind but an abort. The record has none.
#[test]
#[timeout(15000)]
fn a_block_on_taking_the_seat_as_a_thread_ends_does_not_abort() {
    /// Keeps a runtime alive past the thread that built it, and drives it once more as it goes.
    struct DrivesAsItGoes(Option<SharedRuntime>);

    impl Drop for DrivesAsItGoes {
        fn drop(&mut self) {
            crate::block_on(async { drop(SharedRuntime::current().unwrap()) });
        }
    }

    thread_local! {
        static DRIVES_AS_IT_GOES: RefCell<DrivesAsItGoes> =
            const { RefCell::new(DrivesAsItGoes(None)) };
    }

    let thread = thread::spawn(|| {
        crate::block_on(async {
            // Touched before the runtime is built, so that this value is destroyed after any
            // local the lookup inside `SharedRuntime::current` brings into being. The handle it
            // keeps is what the registry's weak reference still resolves to as the
            // thread ends.
            DRIVES_AS_IT_GOES
                .with(|drives| drives.borrow_mut().0 = Some(SharedRuntime::current().unwrap()));
        });
    });

    assert!(thread.join().is_ok());
}

#[test]
#[timeout(15000)]
fn a_runtime_made_by_new_starts_no_helper() {
    let runtime = SharedRuntime::new().unwrap();
    let ran = Arc::new(AtomicBool::new(false));
    let task = {
        let ran = ran.clone();
        runtime.spawn("a task that names the thread it runs on", async move {
            ran.store(true, Ordering::Release);
            thread::current().id()
        })
    };
    let (source, _peer) = pair();
    let _registration = runtime.register(source).unwrap();

    // Long enough that a helper, had one been started, would have run the task.
    thread::sleep(Duration::from_millis(200));
    assert!(!ran.load(Ordering::Acquire));

    // The task waits for the next thread to drive the runtime, and runs on that thread.
    assert_eq!(runtime.block_on(task).unwrap(), thread::current().id());
    assert!(ran.load(Ordering::Acquire));
}

#[test]
#[timeout(15000)]
fn current_inside_a_local_block_on_is_run_by_a_helper() {
    let local = LocalRuntime::new().unwrap();

    // The local runtime's `block_on` drives that runtime alone, so what is built on the one
    // `SharedRuntime::current` hands out here has a helper to run it rather than this thread.
    let name = local
        .block_on(async {
            SharedRuntime::current()
                .unwrap()
                .spawn("a task that names the thread it runs on", async {
                    thread::current().name().map(str::to_owned)
                })
                .await
        })
        .unwrap();

    assert_eq!(name.as_deref(), Some("zruntime"));
}

/// A task spawned with no runtime to name, from a thread that drives none, goes on the runtime
/// the whole process shares, as what `SharedRuntime::current` builds does, and a helper thread
/// runs it.
#[test]
#[timeout(15000)]
fn a_spawn_outside_any_block_on_goes_to_the_process_runtime() {
    // Held across the spawn, so that the runtime the process shares is alive for the comparison
    // and is the same one throughout.
    let process = SharedRuntime::current().unwrap();

    let task = crate::spawn(async {
        // Built on a helper thread, which is in the seat of the runtime it runs.
        let runtime = SharedRuntime::current().unwrap();

        (
            Arc::as_ptr(runtime.inner()) as usize,
            thread::current().name().map(str::to_owned),
        )
    });
    let (built_on, name) = block_on(task).unwrap();

    assert_eq!(built_on, Arc::as_ptr(process.inner()) as usize);
    assert_eq!(name.as_deref(), Some("zruntime"));
}

/// A task spawned with no runtime to name, from inside the free `block_on`, goes on the thread's
/// own runtime, which the thread inside the call runs: it needs no helper.
#[test]
#[timeout(15000)]
fn a_spawn_inside_block_on_goes_to_the_threads_own_runtime() {
    let (ran_on, own) = crate::block_on(async {
        let ran_on = crate::spawn(async { thread::current().id() })
            .await
            .unwrap();

        (ran_on, SharedRuntime::current().unwrap())
    });

    assert_eq!(ran_on, thread::current().id());
    assert!(!own.helper_running());
}

/// A task spawned with no runtime to name, inside the free `block_on` run from the destructor of
/// a thread-local as its thread ends, goes on the thread's own runtime and runs to its end, as it
/// does anywhere else, though the spawn is what brings that runtime into being.
///
/// The thread drives a runtime out of a registry once the value is in place, so that the list of
/// what each of its `block_on` calls drives is first reached after that value: thread-local
/// destructors run in reverse order of registration, and that list, which has a destructor of its
/// own, is gone by the time the value's runs. Nothing else holds the runtime the spawn brings into
/// being, so what the call holds it with has to outlast that list. A panic out of a destructor
/// there is an abort, so the destructor catches what panics and hands it on.
#[test]
#[timeout(15000)]
fn a_spawn_inside_block_on_as_a_thread_ends_runs_on_the_threads_own_runtime() {
    /// Spawns inside the free `block_on` as it is dropped, and sends what came of it.
    struct SpawnsAsItGoes(Option<mpsc::Sender<Result<io::Result<u32>, String>>>);

    impl Drop for SpawnsAsItGoes {
        fn drop(&mut self) {
            let Some(spawned) = self.0.take() else {
                return;
            };
            let outcome = std::panic::catch_unwind(|| {
                crate::block_on(async { crate::spawn(async { 7 }).await })
            })
            .map_err(|panic| format!("{panic:?}"));
            let _ = spawned.send(outcome);
        }
    }

    thread_local! {
        static SPAWNS_AS_IT_GOES: RefCell<SpawnsAsItGoes> =
            const { RefCell::new(SpawnsAsItGoes(None)) };
    }

    let (sender, spawned) = mpsc::channel();
    let thread = thread::spawn(move || {
        SPAWNS_AS_IT_GOES.with(|spawns| spawns.borrow_mut().0 = Some(sender));
        SharedRuntime::current().unwrap().block_on(async {});
    });

    assert!(thread.join().is_ok());
    let outcome = spawned.recv().unwrap().expect("no panic");
    assert_eq!(outcome.map_err(|error| error.to_string()), Ok(7));
}

/// A thread that leaves the seat it took inside the free `block_on` is off the record of the
/// runtime it drives once the call returns: what it spawns or builds from then on goes on the
/// runtime the process shares, as on any thread that drives none, rather than on the one it has
/// stopped running.
#[test]
#[timeout(15000)]
fn a_thread_leaving_a_seat_is_off_the_record() {
    // Held across the spawn, so that the runtime the process shares is alive for the comparison
    // and is the same one throughout.
    let process = SharedRuntime::current().unwrap();

    // Held past the call, so that a record left standing would still name a live runtime.
    let own = crate::block_on(async {
        // Brought into being here, after the call has looked for a runtime to take the seat of
        // and found none: the yield gives the call the turn of its loop on which it takes it.
        let own = SharedRuntime::current().unwrap();
        yield_now().await;
        let driven = Shared::driven().expect("the runtime this thread is in the seat of");
        assert!(Arc::ptr_eq(&driven, own.inner()));

        own
    });

    assert!(Shared::driven().is_none());
    assert!(Arc::ptr_eq(
        SharedRuntime::current().unwrap().inner(),
        process.inner()
    ));
    let task =
        crate::spawn(async { Arc::as_ptr(SharedRuntime::current().unwrap().inner()) as usize });
    assert_eq!(
        block_on(task).unwrap(),
        Arc::as_ptr(process.inner()) as usize
    );
    drop(own);
}

/// A task spawned with no runtime to name, from inside a task the helper is running, goes on the
/// runtime the helper runs. The runtime here is in no registry, so that the process's is not it.
#[test]
#[timeout(15000)]
fn a_spawn_from_a_task_the_helper_runs_goes_on_its_runtime() {
    let runtime = runtime();
    let expected = Arc::as_ptr(runtime.inner()) as usize;

    let task = runtime.spawn("a task that spawns another", async {
        crate::spawn(async { Arc::as_ptr(SharedRuntime::current().unwrap().inner()) as usize })
            .await
            .unwrap()
    });

    assert_eq!(block_on(task).unwrap(), expected);
}

/// A spawn inside the `block_on` of a runtime made by `Runtime::new` goes on that runtime, and
/// the thread inside the call runs the task, where `SharedRuntime::current` there is the one the
/// process shares, whose helper runs what is built on it.
#[test]
#[timeout(15000)]
fn a_spawn_inside_a_block_on_on_a_runtime_made_by_new_goes_on_that_runtime() {
    let runtime = SharedRuntime::new().unwrap();
    // Held, so that the runtime the process shares is alive for the comparison.
    let process = SharedRuntime::current().unwrap();

    let (spawned, current, from_current) = runtime.block_on(async {
        let spawned = crate::spawn(async { thread::current().id() })
            .await
            .unwrap();
        let current = SharedRuntime::current().unwrap();
        let from_current = current
            .spawn("a task that names the thread it runs on", async {
                thread::current().name().map(str::to_owned)
            })
            .await
            .unwrap();

        (spawned, current, from_current)
    });

    assert_eq!(spawned, thread::current().id());
    assert!(Arc::ptr_eq(current.inner(), process.inner()));
    assert_eq!(from_current.as_deref(), Some("zruntime"));
}

/// A spawn inside a `block_on` on a local runtime goes on the runtime the process shares, as what
/// `SharedRuntime::current` builds does there, and a helper thread runs it.
#[test]
#[timeout(15000)]
fn a_spawn_inside_a_local_block_on_goes_to_the_process_runtime() {
    let local = LocalRuntime::new().unwrap();
    // Held across the spawn, so that the runtime the process shares is alive for the comparison.
    let process = SharedRuntime::current().unwrap();

    let (built_on, name) = local
        .block_on(async {
            crate::spawn(async {
                let runtime = SharedRuntime::current().unwrap();

                (
                    Arc::as_ptr(runtime.inner()) as usize,
                    thread::current().name().map(str::to_owned),
                )
            })
            .await
        })
        .unwrap();

    assert_eq!(built_on, Arc::as_ptr(process.inner()) as usize);
    assert_eq!(name.as_deref(), Some("zruntime"));
}

/// A spawn inside a `block_on` that does not hold the seat goes on the runtime that call drives,
/// even while the helper is in the seat.
///
/// The call finds the helper in the seat, so its first poll is made without it: the runtime the
/// thread is in the seat of is none, and the one it is inside `block_on` on is the one to find.
#[cfg(feature = "event")]
#[test]
#[timeout(15000)]
fn a_spawn_inside_a_block_on_that_holds_no_seat_goes_on_its_runtime() {
    let runtime = runtime();
    let _held = held_by_the_helper(&runtime);
    let expected = Arc::as_ptr(runtime.inner()) as usize;

    let built_on = runtime.block_on(async {
        crate::spawn(async { Arc::as_ptr(SharedRuntime::current().unwrap().inner()) as usize })
            .await
            .unwrap()
    });

    assert_eq!(built_on, expected);
}

/// What is built inside a `block_on` on a runtime that is not the thread's own goes on that
/// runtime, even while another thread holds its seat.
///
/// The call finds the helper in the seat, so its first poll is made without it. Work built there
/// on the thread's own runtime would ask for no helper, the thread being inside a `block_on`, and
/// wait for a call on that runtime that never comes.
#[cfg(feature = "event")]
#[test]
#[timeout(15000)]
fn work_built_in_a_block_on_on_another_runtime_goes_on_that_runtime() {
    let runtime = runtime();
    let _held = held_by_the_helper(&runtime);

    let (built_on, answer) = runtime.block_on(async {
        let built_on = SharedRuntime::current().unwrap();
        let answer = built_on.spawn("an answer", async { 7 }).await.unwrap();

        (built_on, answer)
    });

    assert!(Arc::ptr_eq(built_on.inner(), runtime.inner()));
    assert_eq!(answer, 7);
}

/// Work handed to the thread's own runtime from inside a `block_on` on another runtime gets a
/// helper.
///
/// The call drives the other runtime, and has no turn of its loop to spare for the thread's own:
/// what is spawned there has nobody but a helper to run it.
#[test]
#[timeout(15000)]
fn work_handed_to_the_own_runtime_inside_another_runtimes_block_on_goes_to_a_helper() {
    let own = crate::block_on(async { SharedRuntime::current().unwrap() });
    let runtime = runtime();

    let name = runtime
        .block_on(async {
            own.spawn("a task that names the thread it runs on", async {
                thread::current().name().map(str::to_owned)
            })
            .await
        })
        .unwrap();

    assert_eq!(name.as_deref(), Some("zruntime"));
}

/// A `block_on` on a runtime of no registry's panics inside the future of a `block_on` of the
/// helper layer, whether or not that call holds its seat at the time.
#[test]
#[timeout(15000)]
fn a_core_block_on_inside_a_helper_block_on_panics_with_or_without_the_seat() {
    let nested = || {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            LocalRuntime::new().unwrap().block_on(async {});
        }))
        .is_err()
    };

    // Nothing is alive for the thread's own registry to name, so the call polls without a seat.
    assert!(crate::block_on(async { nested() }));
    // A runtime the call resolves to from the start, whose seat it takes before its first poll.
    let runtime = runtime();
    assert!(drive(&runtime, async { nested() }));
    // And the thread may drive the local runtime once it is out of both.
    assert_eq!(LocalRuntime::new().unwrap().block_on(async { 7 }), 7);
}

/// A spawn on a runtime whose `block_on` a nested `block_on` keeps from its loop is run.
///
/// The outer call finds the helper in the seat, so it puts itself down as waiting and polls its
/// future without the seat, which is what lets the free `block_on` be called inside it. The helper
/// would give the seat up to that outer call and park, and the outer call cannot take the seat up
/// until the nested one returns; a spawn on the outer call's runtime from inside the nested one
/// would then take the parked helper for one about to be roused, and wait for ever. The nested
/// call takes the outer one off the list instead, and the helper keeps the seat.
#[cfg(feature = "event")]
#[test]
#[timeout(15000)]
fn a_block_on_nested_in_a_waiting_block_on_leaves_its_runtime_to_the_helper() {
    let runtime = runtime();
    let _held = held_by_the_helper(&runtime);

    let answer = runtime.block_on(async {
        crate::block_on(async {
            // Looked at before the spawn, so that a helper that parks for the outer call has
            // parked by the time of it, and that spawn is the one that finds it so.
            assert!(the_helper_parked_or_nobody_waiting(&runtime));
            runtime.spawn("an answer", async { 7 }).await.unwrap()
        })
    });

    assert_eq!(answer, 7);
}

/// The same, with the nested call made on another runtime from `SharedRuntime::current`, whose
/// seat it takes.
#[cfg(feature = "event")]
#[test]
#[timeout(15000)]
fn a_block_on_on_another_runtime_nested_in_a_waiting_one_leaves_its_runtime_to_the_helper() {
    let other = runtime();
    let runtime = runtime();
    let _held = held_by_the_helper(&runtime);

    let answer = runtime.block_on(async {
        other.block_on(async {
            assert!(the_helper_parked_or_nobody_waiting(&runtime));
            runtime.spawn("an answer", async { 7 }).await.unwrap()
        })
    });

    assert_eq!(answer, 7);
}

/// The same, with the outer call one of the free `block_on`, driving the thread's own runtime.
#[cfg(feature = "event")]
#[test]
#[timeout(15000)]
fn a_block_on_nested_in_a_waiting_free_block_on_leaves_the_own_runtime_to_the_helper() {
    let other = runtime();
    let runtime = runtime();
    let _held = held_by_the_helper(&runtime);

    let answer = drive(&runtime, async {
        other.block_on(async {
            assert!(the_helper_parked_or_nobody_waiting(&runtime));
            runtime.spawn("an answer", async { 7 }).await.unwrap()
        })
    });

    assert_eq!(answer, 7);
}

/// A `block_on` nested inside a waiting `block_on` on the very same runtime waits for that seat,
/// and is given it by the helper.
#[cfg(feature = "event")]
#[test]
#[timeout(15000)]
fn a_block_on_nested_in_a_waiting_block_on_on_the_same_runtime_takes_the_seat() {
    let runtime = runtime();
    let _held = held_by_the_helper(&runtime);
    let (parked, watcher) = the_parked_helper_announced(&runtime);

    let ran_on = runtime
        .block_on(async {
            runtime.block_on(async {
                // Awaited before the spawn below, so that the spawn is made by a thread that has
                // the seat: the helper gives it up to the nested call, and stays
                // parked until it leaves.
                parked.await;
                runtime
                    .spawn("a task that names the thread it runs on", async {
                        thread::current().id()
                    })
                    .await
            })
        })
        .unwrap();

    assert!(watcher.join().unwrap());
    assert_eq!(ran_on, thread::current().id());
}

/// A runtime of this test's own, with nothing to do and no thread until it is given something.
fn runtime() -> SharedRuntime {
    SharedRuntime::from_inner(Core::with_seat().unwrap())
}

/// A `block_on` on `runtime`, resolving to that runtime and no other, with that runtime the
/// calling thread's own for as long as the call lasts.
fn drive<F>(runtime: &SharedRuntime, future: F) -> F::Output
where
    F: Future,
{
    let _own = driver::own_for_the_call(runtime.inner());
    let inner = runtime.inner().clone();

    driver::block_on(driver::Target::Own, &move || Some(inner.clone()), future)
}

/// A task of `runtime`'s that never finishes, spawned from outside `block_on` and waited for
/// until the helper thread has taken it up.
///
/// A helper is up the moment a spawn from outside asks for one, but the thread it is on takes a
/// moment to reach the seat. The task announces its first poll, which the helper makes from the
/// seat, and this returns once that announcement is out: the helper is in the seat by then, and
/// what keeps it there until the task is dropped is the task itself.
#[cfg(feature = "event")]
fn held_by_the_helper(runtime: &SharedRuntime) -> Task<(), Shared> {
    let announce = Arc::new(Event::new());
    let announced = announce.listen();
    let task = runtime.spawn("a task that never finishes", async move {
        announce.notify(1);
        pending::<()>().await
    });
    block_on(announced);

    task
}

/// An announcement of `runtime`'s helper being parked: what a `block_on` awaits to know that
/// the seat has been given up to it, and the thread that watches for the park.
///
/// A thread of its own, because the `block_on` this is for is the very thing the helper parks
/// for and has no poll to spare for the watching. The announcement is made whether or not the
/// park is seen, so that a call awaiting it is never left waiting on one that did not happen;
/// whether it did is what the thread hands back where it is joined.
#[cfg(feature = "event")]
fn the_parked_helper_announced(
    runtime: &SharedRuntime,
) -> (EventListener, thread::JoinHandle<bool>) {
    let announce = Arc::new(Event::new());
    // Taken before the thread is started, so that the announcement cannot be missed.
    let announced = announce.listen();
    let runtime = runtime.clone();
    let watcher = thread::spawn(move || {
        let parked = within_a_second(|| runtime.helper_parked());
        announce.notify(1);

        parked
    });

    (announced, watcher)
}

/// A resolver that finds no runtime the first time it is asked and `runtime` every time after,
/// beside a guard that makes `runtime` the calling thread's own for as long as it lives.
///
/// Which is what a `block_on` whose very first poll spawns a task comes to: it polls without the
/// seat, and the work that poll leaves has nobody to run it. The task is spawned on the calling
/// thread, so the runtime is that thread's own from the poll onwards and the spawn that poll
/// makes asks for no helper.
fn resolve_on_the_second_ask(runtime: &SharedRuntime) -> (Resolver, impl Drop) {
    let inner = runtime.inner().clone();
    let looked = AtomicBool::new(false);
    let own = driver::own_for_the_call(runtime.inner());

    let resolve = Box::new(move || {
        if looked.swap(true, Ordering::AcqRel) {
            return Some(inner.clone());
        }

        None
    });

    (resolve, own)
}

/// What a `block_on` looks its runtime up through, boxed so that a test can hand it around.
type Resolver = Box<dyn Fn() -> Option<Arc<Core<Shared>>>>;

/// Whether, within a second, `runtime`'s helper has parked or nobody is down as waiting for its
/// seat.
///
/// For the future of a `block_on` nested inside another on `runtime` that found the helper in the
/// seat: the outer call is down as waiting, and the helper either parks for it or — where the
/// nested call has taken it off the list — is left in the seat. Either way the helper is done
/// deciding by the time this holds, so what follows sees it settled.
#[cfg(feature = "event")]
fn the_helper_parked_or_nobody_waiting(runtime: &SharedRuntime) -> bool {
    within_a_second(|| runtime.helper_parked() || waiting(runtime) == 0)
}

/// How many threads are down as waiting for `runtime`'s seat.
#[cfg(feature = "event")]
fn waiting(runtime: &SharedRuntime) -> usize {
    driver::seat(runtime.inner()).waiting()
}

/// Whether `runtime`'s helper has gone within a second.
fn helper_gone(runtime: &SharedRuntime) -> bool {
    within_a_second(|| !runtime.helper_running())
}

/// Whether `condition` holds within a second, looked at every 10 ms.
fn within_a_second(condition: impl Fn() -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        if condition() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(Duration::from_millis(10));
    }
}

/// A waker that counts how often it has been woken.
#[derive(Default)]
struct Woken(AtomicUsize);

impl Woken {
    /// How often this waker has been woken.
    fn count(&self) -> usize {
        self.0.load(Ordering::Acquire)
    }
}

impl Wake for Woken {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::AcqRel);
    }
}
