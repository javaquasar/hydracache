//! What `Event`'s listen and notify paths cost: taking a listener, alone and dropped at once,
//! sending a notification to nobody, waking one listener and a hundred taken beforehand, and
//! `lock::Mutex` and `lock::Semaphore`, built on an event, taken by one thread alone. `Event` needs
//! no runtime, so no id here goes through `zruntime::block_on`: they poll listeners by hand or take
//! the mutex with `try_lock` and a permit with `try_acquire`, all on one thread.
//! `mutex/4-threads`, the mutex taken by four contending threads, is in `contention.rs`, as only
//! the clock can measure it.
//!
//! `listen` times taking a listener, alone, on an event whose shared state is already allocated:
//! what every wait starts with. The listeners are taken a batch at a time and dropped once the
//! batch has been timed, so the event's queue holds a batch of them at most.
//!
//! `listen-drop` times taking a listener and dropping it at once, on an event whose shared state
//! is already allocated: the common case of a listener whose wait never has to happen because the
//! condition it was taken for already held.
//!
//! `notify-none` times `notify(1)` on an initialised event with nobody listening: the path a
//! lock's release takes when nobody is waiting for it, and so what `notify` costs even where
//! there is nothing to wake.
//!
//! `mutex/uncontended` times taking and releasing zruntime's own `lock::Mutex` on one thread, with
//! nobody else after it: the commonest use of a lock, whose release notifies the mutex's event with
//! nobody listening.
//!
//! `semaphore/uncontended` times taking a permit of zruntime's own `lock::Semaphore` and giving it
//! back on one thread, with nobody else after one: what a semaphore that caps work which seldom
//! reaches the cap costs, whose release notifies its event with nobody listening.
//!
//! `wake-one` times notifying one listener and polling it once to `Ready`: the end of a wait,
//! from being told it is over to finding so, without its start, which `listen` times.
//!
//! `wake-all/100` times waking a hundred listeners with one `notify(usize::MAX)` and polling each
//! to `Ready`: a broadcast to every listener an event has, such as a connection announcing that it
//! has closed.
//!
//! Each iteration of `wake-one` and `wake-all/100` notifies an event of its own, made and listened
//! to before the timing starts. The listeners it polls are then the only ones its notification can
//! reach, however many iterations a harness sets up before it runs them: CodSpeed's CPU simulation
//! sets up two for the one it measures.

use std::{
    future::Future,
    hint::black_box,
    pin::Pin,
    task::{Context, Poll, Waker},
};

use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use zruntime::{
    Event, EventListener,
    lock::{Mutex, Semaphore},
};

/// Times `listen`, `notify` and the poll that resolves a listener, all on a single thread.
fn event_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("event");

    let listen = Event::new();
    // Allocates the event's shared state up front, so the timed batches below never pay for that.
    listen.notify(0);
    group.bench_function("listen", |b| {
        // The listeners each batch takes are dropped once it has been timed.
        b.iter_batched(
            || (),
            |()| listen.listen(),
            BatchSize::NumIterations(LISTENERS_PER_BATCH),
        );
    });

    let listen_drop = Event::new();
    // Allocates the event's shared state up front, so the timed loop below never pays for that.
    listen_drop.notify(0);
    group.bench_function("listen-drop", |b| {
        b.iter(|| drop(black_box(listen_drop.listen())));
    });

    let notify_none = Event::new();
    notify_none.notify(0);
    group.bench_function("notify-none", |b| {
        b.iter(|| black_box(notify_none.notify(1)));
    });

    let uncontended = Mutex::new(());
    group.bench_function("mutex/uncontended", |b| {
        // The guard is dropped at once: the mutex is taken and released.
        b.iter(|| drop(black_box(uncontended.try_lock())));
    });

    let semaphore = Semaphore::new(1);
    group.bench_function("semaphore/uncontended", |b| {
        // The guard is dropped at once: the permit is taken and given back.
        b.iter(|| drop(black_box(semaphore.try_acquire())));
    });

    group.bench_function("wake-one", |b| {
        // The event and its listener are handed back, to be dropped once the batch has been timed.
        b.iter_batched(
            || listened_to(1),
            |(event, mut listeners)| {
                assert_eq!(event.notify(1), 1);
                assert!(poll_once(&mut listeners[0]).is_ready());

                (event, listeners)
            },
            BatchSize::NumIterations(LISTENERS_PER_BATCH),
        );
    });

    group.throughput(Throughput::Elements(100));
    group.bench_function("wake-all/100", |b| {
        b.iter_batched(
            || listened_to(100),
            |(event, mut listeners)| {
                assert_eq!(event.notify(usize::MAX), 100);
                for listener in &mut listeners {
                    assert!(poll_once(listener).is_ready());
                }

                (event, listeners)
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

/// A new event, and `count` listeners taken on it, the oldest first.
fn listened_to(count: usize) -> (Event, Vec<EventListener>) {
    let event = Event::new();
    let listeners = (0..count).map(|_| event.listen()).collect();

    (event, listeners)
}

/// Polls `listener` once, with a waker that goes nowhere, and hands back what the poll found.
fn poll_once(listener: &mut EventListener) -> Poll<()> {
    Pin::new(listener).poll(&mut Context::from_waker(Waker::noop()))
}

/// How many iterations of `listen` and `wake-one` each timed batch runs: few enough to keep the
/// queue of the event `listen` takes its listeners on short, and enough that reading the clock once
/// a batch costs little next to them.
const LISTENERS_PER_BATCH: u64 = 64;

criterion_group!(benches, event_benches);
criterion_main!(benches);
