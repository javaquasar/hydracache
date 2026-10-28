//! What the runtime costs: a timer, watching a socket, and bytes moved through the reactor's
//! readiness path, within one thread and across two. What spawning a task costs is in `spawn.rs`.
//! These are the runtime's share of
//! what a connection built on it costs: the ids mirror what zbus measures of its connections (a
//! connection's setup and teardown, a method call's round trip, a large body, a burst of
//! concurrent calls to a peer on another thread) with the protocol taken out.
//!
//! Each id times its operation inside one `zruntime::block_on`, the shape of a program that does
//! all of its work inside one call. The runtime handle a routine spawns tasks or timers on is
//! taken once, in a `block_on` of its own, before the group starts, so only the operation runs
//! inside the timed `block_on`; a `SharedRuntime` handle kept alive this way is also what a later
//! `block_on` on the same thread resolves to, rather than a fresh runtime.
//!
//! `io/roundtrip-among-256-idle` is `io/roundtrip` again, while the runtime watches 256 sockets
//! nothing is sent on, each with a task waiting to read from it: what a wait costs per watched
//! socket, which a poller that keeps what it watches in the kernel leaves out.
//!
//! The `timer` ids need nothing but the reactor's timers, so they run on every platform. The `io`
//! and `cross-thread` ids need a unix socket pair to give the reactor's readiness path something to
//! watch, so they are unix-only.

use std::{
    future::{Future, poll_fn},
    pin::Pin,
    task::Poll,
    time::Duration,
};

use criterion::{Criterion, Throughput, criterion_group};
use executor::{ZruntimeExecutor, runtime_handle};
use zruntime::{Shared, Sleep};

#[cfg(unix)]
#[path = "common/cpus.rs"]
mod cpus;
#[path = "common/executor.rs"]
mod executor;

fn timer_benches(c: &mut Criterion) {
    let runtime = runtime_handle();

    // Each iteration waits out a real timer, so a sample of the default size already takes a
    // handful of milliseconds; kept small so the group stays quick without losing the signal.
    let mut group = c.benchmark_group("timer");
    group.sample_size(20);
    group.bench_function("sleep-1ms", |b| {
        b.to_async(ZruntimeExecutor)
            .iter(|| async { runtime.sleep(TIMER_DURATION).await });
    });
    group.throughput(Throughput::Elements(100));
    group.bench_function("100-timers", |b| {
        b.to_async(ZruntimeExecutor).iter(|| async {
            let sleeps: Vec<_> = (0..100).map(|_| runtime.sleep(TIMER_DURATION)).collect();
            join_all(sleeps).await
        });
    });
    group.finish();
}

/// How long each timer benchmark arms its `Sleep`s for: long enough that the first poll, right
/// after arming, still finds the deadline ahead of it. `Duration::ZERO` would already be due by
/// that poll and resolve on the spot, which would measure the clock rather than the reactor's
/// timer path.
const TIMER_DURATION: Duration = Duration::from_millis(1);

/// Awaits every one of `sleeps`, polling all of them together on each wake so that they run
/// concurrently rather than one after another.
async fn join_all(mut sleeps: Vec<Sleep<Shared>>) {
    poll_fn(move |cx| {
        sleeps.retain_mut(|sleep| Pin::new(sleep).poll(cx).is_pending());

        if sleeps.is_empty() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    })
    .await
}

#[cfg(unix)]
mod unix {
    use std::{
        future::poll_fn,
        hint::black_box,
        io::{Read, Write},
        os::unix::net::UnixStream,
        sync::Arc,
        thread,
    };

    use criterion::{BatchSize, Criterion, Throughput};
    use futures::future;
    use zruntime::{Interest, Registration, Shared, SharedRuntime};

    use super::{ZruntimeExecutor, cpus::Cpus, runtime_handle};

    const BIG: usize = 1024 * 1024;
    /// How many requests `cross-thread/1000-in-flight` has in flight at once.
    const IN_FLIGHT: usize = 1000;
    /// How much of a large body one read or write moves at a time, so the echo task and the
    /// bench routine trade turns with the reactor rather than one of them running to completion
    /// in a single syscall.
    const CHUNK: usize = 64 * 1024;
    /// How many socket pairs `io/register-and-drop` makes ahead of each batch of iterations.
    const REGISTER_BATCH: u64 = 64;
    /// How many idle sockets `io/roundtrip-among-256-idle` watches beside the round trip: a pair
    /// each, which keeps all of them within a default limit of 1024 open descriptors.
    const IDLE: usize = 256;

    pub(super) fn io_benches(c: &mut Criterion) {
        let runtime = runtime_handle();
        let mut group = c.benchmark_group("io");
        // A fresh pair for every iteration, made outside the timed routine, so that only
        // putting both ends under the reactor's watch and taking them off it again is timed.
        // Every pair of a batch is made up front and held open until the batch is done, so the
        // batch is kept to a fixed size: `SmallInput` scales it with the iteration count, and
        // at tens of thousands of pairs it runs past a default limit of 1024 open descriptors.
        group.bench_function("register-and-drop", |b| {
            let runtime = &runtime;
            b.to_async(ZruntimeExecutor).iter_batched(
                pair,
                |(local, far)| async move {
                    let registrations = (register(runtime, &local), register(runtime, &far));
                    drop(black_box(registrations));
                },
                BatchSize::NumIterations(REGISTER_BATCH),
            );
        });
        {
            let (local, far) = pair();
            let registration = register(&runtime, &local);
            let far_registration = register(&runtime, &far);
            let _echo = runtime.spawn("bench echo", echo_forever(far_registration, far));
            group.bench_function("roundtrip", |b| {
                b.to_async(ZruntimeExecutor).iter(|| async {
                    write_all(&registration, &local, &[7]).await;
                    let mut response = [0u8; 1];
                    read_exact(&registration, &local, &mut response).await;
                    black_box(response);
                });
            });
        }
        {
            // The round trip above, while the runtime watches as many sockets that nothing is
            // sent on: a server's idle connections, each with a task waiting to read from it.
            // A wait on a poller that keeps what it watches in the kernel costs what the ready
            // sockets cost, and one on `poll(2)` a look at every watched socket.
            let _idle: Vec<_> = (0..IDLE)
                .map(|_| {
                    let (near, far) = pair();
                    let registration = runtime.register(near).unwrap();
                    let waiting = runtime.spawn("bench idle", async move {
                        registration.ready(Interest::Readable).await.unwrap();
                    });

                    (waiting, far)
                })
                .collect();
            let (local, far) = pair();
            let registration = register(&runtime, &local);
            let far_registration = register(&runtime, &far);
            let _echo = runtime.spawn("bench echo", echo_forever(far_registration, far));
            group.bench_function("roundtrip-among-256-idle", |b| {
                b.to_async(ZruntimeExecutor).iter(|| async {
                    write_all(&registration, &local, &[7]).await;
                    let mut response = [0u8; 1];
                    read_exact(&registration, &local, &mut response).await;
                    black_box(response);
                });
            });
        }
        group.sample_size(10);
        group.throughput(Throughput::Bytes(BIG as u64));
        {
            let (local, far) = pair();
            let registration = register(&runtime, &local);
            let far_registration = register(&runtime, &far);
            let _echo = runtime.spawn("bench echo", echo_forever(far_registration, far));
            let body = vec![7u8; BIG];
            // The body is written while the echo is read back, as neither side of the pair can
            // be counted on to buffer all of it: the echo task replies to each chunk before it
            // reads the next, so writing the whole body first would leave both sides waiting on
            // each other once their buffers filled.
            group.bench_function("1MiB", |b| {
                b.to_async(ZruntimeExecutor).iter(|| async {
                    let mut response = vec![0u8; BIG];
                    future::join(
                        write_all(&registration, &local, &body),
                        read_exact(&registration, &local, &mut response),
                    )
                    .await;
                    black_box(response);
                });
            });
        }
        group.finish();
    }

    pub(super) fn cross_thread_benches(c: &mut Criterion) {
        let mut group = c.benchmark_group("cross-thread");
        group.sample_size(10);

        // The two threads are pinned to a CPU each, for as long as the group runs, so that every
        // process measures the same placement of the two (see `common/cpus.rs`).
        let cpus = Cpus::allowed();
        let far_cpu = cpus.nth(1);
        let _pinned = cpus.nth(0).pin();

        let (local, far) = pair();
        let far_thread = thread::spawn(move || {
            let _pinned = far_cpu.pin();
            zruntime::block_on(async move {
                let runtime = SharedRuntime::current().expect("a runtime for this thread");
                let far_registration = register(&runtime, &far);
                echo_forever(far_registration, far).await;
            });
        });

        let runtime = runtime_handle();
        let local = Arc::new(Watched {
            registration: register(&runtime, &local),
            stream: local,
        });
        group.bench_function("roundtrip", |b| {
            b.to_async(ZruntimeExecutor).iter(|| async {
                write_all(&local.registration, &local.stream, &[7]).await;
                let mut response = [0u8; 1];
                read_exact(&local.registration, &local.stream, &mut response).await;
                black_box(response);
            });
        });
        // Every request is written on its own by a spawned task while the routine reads the
        // replies, so up to all of them are in flight at once and both sides of the socket are
        // waited on together, as a burst of concurrent calls to a peer does.
        group.throughput(Throughput::Elements(IN_FLIGHT as u64));
        group.bench_function("1000-in-flight", |b| {
            b.to_async(ZruntimeExecutor).iter(|| async {
                let writer = {
                    let local = local.clone();
                    runtime.spawn("bench writer", async move {
                        for request in 0..IN_FLIGHT as u32 {
                            write_all(&local.registration, &local.stream, &request.to_le_bytes())
                                .await;
                        }
                    })
                };
                let mut replies = vec![0u8; IN_FLIGHT * size_of::<u32>()];
                read_exact(&local.registration, &local.stream, &mut replies).await;
                writer.await.unwrap();
                black_box(replies);
            });
        });

        group.finish();
        // Closing the local side is what lets the far thread's next read see the end of the
        // stream and return, ending its `block_on` so the join below does not hang.
        drop(local);
        far_thread.join().unwrap();
    }

    /// One end of a pair together with its registration, shared between the routine and a task
    /// it spawns.
    struct Watched {
        registration: Registration<Shared>,
        stream: UnixStream,
    }

    /// Watches `stream` on `runtime` through a clone of its descriptor, so the registration can
    /// be dropped on its own while `stream` stays open for the direct reads and writes below.
    fn register(runtime: &SharedRuntime, stream: &UnixStream) -> Registration<Shared> {
        runtime.register(stream.try_clone().unwrap()).unwrap()
    }

    /// Two connected, nonblocking ends of a unix socket pair.
    fn pair() -> (UnixStream, UnixStream) {
        let (a, b) = UnixStream::pair().unwrap();
        for stream in [&a, &b] {
            stream.set_nonblocking(true).unwrap();
        }

        (a, b)
    }

    /// Reads whatever bytes arrive on `far` and writes them straight back, forever. The far end
    /// of every echo the benchmarks above drive, from a spawned task or another thread's
    /// `block_on`. Returns once the near side has closed, which is when a read comes back empty.
    async fn echo_forever(registration: Registration<Shared>, far: UnixStream) {
        let mut chunk = [0u8; CHUNK];
        loop {
            let read = read_some(&registration, &far, &mut chunk).await;
            if read == 0 {
                return;
            }
            write_all(&registration, &far, &chunk[..read]).await;
        }
    }

    /// Reads at least one byte from `stream` into `buf`, or `0` once the peer's side has closed.
    async fn read_some(
        registration: &Registration<Shared>,
        mut stream: &UnixStream,
        buf: &mut [u8],
    ) -> usize {
        poll_fn(|cx| registration.poll_io(cx, Interest::Readable, || stream.read(buf)))
            .await
            .unwrap()
    }

    /// Reads until `buf` is full, waiting out `WouldBlock` on `registration` in between.
    async fn read_exact(
        registration: &Registration<Shared>,
        stream: &UnixStream,
        mut buf: &mut [u8],
    ) {
        while !buf.is_empty() {
            let read = read_some(registration, stream, buf).await;
            assert_ne!(read, 0, "the peer closed the connection early");
            buf = &mut buf[read..];
        }
    }

    /// Writes every byte of `buf` to `stream`, waiting out `WouldBlock` on `registration` in
    /// between.
    async fn write_all(
        registration: &Registration<Shared>,
        mut stream: &UnixStream,
        mut buf: &[u8],
    ) {
        while !buf.is_empty() {
            let written =
                poll_fn(|cx| registration.poll_io(cx, Interest::Writable, || stream.write(buf)))
                    .await
                    .unwrap();
            buf = &buf[written..];
        }
    }
}

#[cfg(unix)]
criterion_group!(
    benches,
    timer_benches,
    unix::io_benches,
    unix::cross_thread_benches
);
#[cfg(not(unix))]
criterion_group!(benches, timer_benches);

criterion::criterion_main!(benches);
