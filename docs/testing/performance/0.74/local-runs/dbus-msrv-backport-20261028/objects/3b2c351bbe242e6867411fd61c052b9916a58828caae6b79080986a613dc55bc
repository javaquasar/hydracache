<p align="center">
  <a href="https://github.com/z-galaxy/zruntime/actions/workflows/rust.yml">
    <img alt="CI Pipeline Status" src="https://github.com/z-galaxy/zruntime/actions/workflows/rust.yml/badge.svg">
  </a>
  <a href="https://docs.rs/zruntime/">
    <img alt="API Documentation" src="https://docs.rs/zruntime/badge.svg">
  </a>
  <a href="https://crates.io/crates/zruntime">
    <img alt="crates.io" src="https://img.shields.io/crates/v/zruntime">
  </a>
  <a href="https://app.codspeed.io/z-galaxy/zruntime?utm_source=badge">
    <img alt="CodSpeed" src="https://img.shields.io/endpoint?url=https://codspeed.io/badge.json">
  </a>
</p>

<p align="center">
  <img alt="zruntime logo" src="https://raw.githubusercontent.com/z-galaxy/zruntime/f7ba9e0ca073c3d2981bbb443a74ad62a863255e/data/logo.svg">
</p>

<h1 align="center">zruntime</h1>

A simple, single-threaded async runtime for Rust.

A [`Runtime`] runs async tasks, timers and I/O on a single thread. [`Runtime::block_on`] runs a
future to completion on the calling thread. While it waits for the future, the same thread also
runs the runtime's tasks, timers and I/O. A runtime comes in two flavours:

* [`LocalRuntime`] is the default. It stays on the thread that created it, and runs any `'static`
  future, `Send` or not. Prefer it unless the runtime, or a future built on it, has to cross
  threads or be polled by another executor.
* [`SharedRuntime`] can be used from, and run on, any thread. It runs `Send` futures only.

The flavour is a type parameter: [`LocalRuntime`] is `Runtime<Local>`, and [`SharedRuntime`] is
`Runtime<Shared>`. A runtime is cheap to clone, and all its clones refer to the same runtime.

## Overview

The runtime, and what runs on it:

* **Runtimes**: [`LocalRuntime`] and [`SharedRuntime`], each run by [`Runtime::block_on`]. With
  the `helper` feature, the free [`block_on`] runs a future on the calling thread's own runtime,
  which [`SharedRuntime::current`] returns.
* **Tasks**: [`Runtime::spawn`], or the free [`spawn_local`] and [`spawn`] for code that has no
  runtime handle. Each returns a [`Task`].
* **Timers**: [`Runtime::sleep`], [`Runtime::timeout`] and [`Runtime::interval`]. Each has a
  variant that takes a point in time: [`Runtime::sleep_until`], [`Runtime::timeout_at`] and
  [`Runtime::interval_at`].
* **Sockets**: `TcpListener`, `TcpStream` and `UdpSocket` in the [`net`] module, and the
  unix-domain sockets in its `unix` module, on unix only.
* **Other I/O sources**: [`AsyncIo`], the async handle of a file descriptor or socket.
* **Child processes**: the [`process`] module.

What needs no runtime, and works under any executor:

* **Blocking work**: [`unblock()`] and [`Unblock`].
* **Files**: the [`fs`] module.
* **Events, locks and channels**: [`Event`], the [`lock`] module, [`mpmc`] and [`broadcast`].

The runtime and [`Event`] are on by default. The rest is behind [cargo features](#cargo-features).

## Examples

A local runtime, used by the thread that created it:

```rust
use std::{cell::RefCell, rc::Rc, time::Duration};

use zruntime::LocalRuntime;

let runtime = LocalRuntime::new().expect("a runtime for this thread");
let doubled = runtime.block_on(async {
    let count = Rc::new(RefCell::new(0));
    let sleeper = runtime.clone();
    let task = runtime.spawn("double", {
        let count = count.clone();
        async move {
            sleeper.sleep(Duration::from_millis(1)).await;
            *count.borrow_mut() += 21;
            *count.borrow() * 2
        }
    });

    task.await.expect("the task did not panic")
});

assert_eq!(doubled, 42);
```

A shared runtime, spawned on from another thread and run by the main thread:

```rust
use std::{sync::mpsc, thread};

use zruntime::SharedRuntime;

let runtime = SharedRuntime::new().expect("a runtime for this thread");
let (sender, receiver) = mpsc::channel();
let spawner = thread::spawn({
    let runtime = runtime.clone();
    move || {
        let task = runtime.spawn("double", async { 21 * 2 });
        sender.send(task).expect("the main thread is still waiting for it");
    }
});

let task = receiver.recv().expect("the other thread sent the task");
let doubled = runtime.block_on(task).expect("the task did not panic");
spawner.join().expect("the other thread did not panic");

assert_eq!(doubled, 42);
```

Create a runtime with `LocalRuntime::new()` or `SharedRuntime::new()`. The compiler cannot infer
the flavour of a plain `Runtime::new()`.

## Tasks

[`Runtime::spawn`] starts a task on the runtime. It takes a name, which the log message uses if
the task panics.

Code that has no runtime handle can use the free [`spawn_local`] and [`spawn`] functions instead.
They spawn on the runtime that is running the calling code, and name the task after the place it
was spawned from.

```rust
use zruntime::{LocalRuntime, Task};

// Has no runtime handle: it spawns on the runtime that runs its caller.
fn double(number: u32) -> Task<u32> {
    zruntime::spawn_local(async move { number * 2 })
}

let runtime = LocalRuntime::new().expect("a runtime for this thread");
let doubled = runtime.block_on(async { double(21).await.expect("the task did not panic") });

assert_eq!(doubled, 42);
```

* [`spawn_local`] takes any `'static` future. Call it from code running on a [`LocalRuntime`]:
  from the future passed to its `block_on`, or from one of its tasks.
* [`spawn`] takes a `Send` future, and spawns it on the [`SharedRuntime`] that runs the calling
  code. If there is none, it panics, unless the `helper` feature is on (see
  [Per-thread runtimes](#per-thread-runtimes)).

Both return a [`Task`], a handle to the task:

* Awaiting it returns the task's output, or an error if the task panicked or its runtime was
  dropped.
* Dropping it cancels the task.
* [`Task::cancel`] cancels the task, and returns a future that completes once it has stopped:
  with the output, if the task had already finished.
* [`Task::detach`] lets the task run on without a handle.
* [`Task::is_finished`] tells whether the task has ended, without taking its output.

Tasks take turns, in the order they were woken. A task keeps the thread until its poll returns,
and the poll of a task whose I/O is always ready, such as a loop that reads from a socket that
already holds the bytes, would never return `Pending` on its own. It would hold up every other
task, and the reactor, for as long as the loop lasts. So a runtime gives every poll a budget of 128
I/O operations, and each operation that completes spends one. Once the budget is gone, the next
operation, and every one after it in that poll, wakes its own task and returns `Pending` instead of
running. That puts the task at the back of the queue: the tasks that were woken before it have
their turn then, and the reactor has its own once the batch of polls the task is in is over. The
task's next poll starts with a full budget. The future passed to [`Runtime::block_on`] or to the
free [`block_on`] has the same budget, because the tasks run only between its polls.

The sockets of the [`net`] module, [`AsyncIo`], [`Registration::poll_io`], the operations of the
[`fs`] module and, on unix, the pipes to a child process spend the budget. Only an operation that
completes counts: one that fails with `WouldBlock` spends nothing, and neither does waiting for
readiness alone. The budget is for I/O alone, so a task that computes, or blocks, still holds the
thread until its poll returns. A thread that is inside no poll by a runtime of this crate has no
budget: a future polled there by another executor, or by plain code, never yields for it.

The budget belongs to the thread, not to the future, so a future that another executor polls
inside a task shares what is left of the task's budget. An executor that blocks the thread until its
future is done, such as `futures::executor::block_on`, polls again as soon as an operation that
yielded has woken it, inside the same poll of the task, where the operation yields again. Once the
budget is spent, such an executor spins, and never gets its future done. So it has to run its
future inside [`unconstrained`], which lifts the budget for each poll of that future and leaves the
task's budget as it was. The task then holds the thread, and every other task, until the future is
done. Awaiting the future in the task, or spawning it, lets the other tasks run meanwhile.

## Timers

* [`Runtime::sleep`] waits for a duration, and [`Runtime::sleep_until`] waits until a deadline.
* [`Sleep::reset`] moves the deadline of a sleep, for example to restart an idle timeout.
* [`Runtime::timeout`] puts a time limit on any future, and [`Runtime::timeout_at`] a deadline.
* [`Runtime::interval`] ticks once every period, and [`Runtime::interval_at`] from a start time.
  The [`Interval`] they return is also a `Stream`.

```rust
use std::{future, time::Duration};

use zruntime::LocalRuntime;

let runtime = LocalRuntime::new().expect("a runtime for this thread");
runtime.block_on(async {
    let mut interval = runtime.interval(Duration::from_millis(1));
    for _ in 0..3 {
        interval.tick().await;
    }

    let never = runtime.timeout(Duration::from_millis(2), future::pending::<()>());
    assert!(never.await.is_err());
});
```

## Sockets

The [`net`] module has TCP and UDP sockets: `TcpListener`, `TcpStream` and `UdpSocket`. On unix,
its `unix` module has `UnixListener`, `UnixStream` and `UnixDatagram`. They are like the sockets of
`smol::net`, and work on both flavours of runtime. Streams implement `AsyncRead` and `AsyncWrite`
from `futures-io`. Connecting never blocks the thread.

The TCP and UDP sockets take socket addresses, not host names. To look up a name without blocking
the thread, run the lookup with [`unblock()`].

## Other I/O sources

[`AsyncIo`] makes any source the runtime can watch async, like `smol::Async`. On unix, that is
anything with a file descriptor: a pipe, a terminal, an eventfd, an inotify instance, or a socket
type that [`net`] does not cover. On Windows, it can only be a socket. [`AsyncIo::new`] takes the
runtime and the source, and switches the source to non-blocking mode.

* [`AsyncIo::read_with`] and [`AsyncIo::write_with`] run an operation on the source until it no
  longer fails with `WouldBlock`.
* [`AsyncIo::readable`] and [`AsyncIo::writable`] only wait for readiness. They are useful when
  another library does the I/O on the descriptor.
* `AsyncIo<T>` implements `AsyncRead` from `futures-io` if `&T` implements `Read`, and
  `AsyncWrite` if `&T` implements `Write`.

Any number of tasks can wait through the four methods at once, but only one task per direction
through the `AsyncRead` and `AsyncWrite` impls.

Running an operation through [`AsyncIo::read_with`], [`AsyncIo::write_with`] or the `AsyncRead` and
`AsyncWrite` impls counts against the budget that [Tasks](#tasks) describes. Waiting with
[`AsyncIo::readable`] or [`AsyncIo::writable`] does not.

`AsyncIo` is not for regular files: no poller can say whether a read or a write of one would wait
for the disk, so either can block the thread. Use the [`fs`] module for files, which runs each
operation on the thread that polls it (see [Files](#files)), or [`Unblock`], which keeps the thread
free.

For code that does its own polling, [`Runtime::register`] watches a source and returns a
[`Registration`] to poll it through. Its [`Registration::poll_io`] spends the budget the same way.

## Blocking work

[`unblock()`] runs blocking work on a pool of threads, and returns a future of its result.
[`Unblock`] wraps a blocking I/O handle, such as a file or the standard input. It implements the
async I/O traits by running each operation on that pool.

Neither needs a runtime: they work under any executor. Neither is built on WASI, which has no
thread to run the work on (see [WASI](#wasi)).

## Files

The [`fs`] module is async filesystem access, in the shape of `std::fs` and like `smol::fs`. It
needs no runtime: it works under any executor, and on WASI (see [WASI](#wasi)).

Unlike [`unblock()`], it runs each operation on the thread that polls its future, within that poll.
Waiting for readiness would not help: a poller either cannot watch a regular file, or reports it as
ready whether or not the disk has the data. Most operations are served from the cache of the OS and
take microseconds. But one that misses the cache, one on a slow or network mount, and a single read
or write of a large buffer block the thread while they run, and every task on it waits meanwhile.
Each operation spends the budget that [Tasks](#tasks) describes, so a task in a loop of them lets
the others run after 128 of them, but the budget does not cut a single operation short.

That matters where other tasks must stay responsive while a large file, or one on a slow disk or
mount, is read or written. There, run the `std::fs` call as [blocking work](#blocking-work)
instead: through [`unblock()`], or through an [`Unblock`] around a `std::fs::File`.

## Child processes

The [`process`] module runs child processes, like `smol::process`. Its `Command` works like
`std::process::Command`, but spawns the child on a runtime that you pass to it. The `Child` it
returns has async `status` and `output` methods. The pipes to the child, `ChildStdin`,
`ChildStdout` and `ChildStderr`, implement `AsyncWrite` and `AsyncRead`.

Reading or writing a pipe, and waiting for the child to exit, never block the thread. Dropping a
`Child` leaves the process running, unless `kill_on_drop(true)` was set on its `Command`. The
[`process`] module documentation has the details.

## Events, locks and channels

These work under any executor, not only zruntime's. Tasks on different runtimes and threads can
share them.

* [`Event`] is a notification that tasks can wait for. A task gets an [`EventListener`] from the
  event and awaits it. Code that changes what the task waits for, for example by releasing a lock,
  notifies the event. That wakes as many listeners as it asks for.
* The [`lock`] module has an async `Mutex`, `RwLock` and `Semaphore`, whose guards can be held
  across an `.await`. It also has a `Barrier`, and a `OnceCell` whose initialiser can be async.
* [`mpmc`] is a multi-producer, multi-consumer channel, bounded or unbounded. Each message goes to
  one receiver.
* [`broadcast`] is a multi-producer, multi-consumer broadcast channel. Each message goes to every
  receiver.

## Running on several threads

One thread at a time runs a runtime, so all its tasks share one CPU core. A task that keeps the
CPU busy holds up every other task on the same runtime. Two threads cannot run one runtime
together: a second `block_on` on a runtime from `SharedRuntime::new` panics, and one on a runtime
from [`SharedRuntime::current`] waits for its turn.

To use more cores, use several runtimes, one per thread, each run by its own `block_on`. With the
`helper` feature, the free [`block_on`] already works like this: each thread that calls it runs
its own runtime.

To spread work over the threads, give each thread a clone of the same [`mpmc`] receiver. Each
piece of work goes to whichever thread receives it first, as [the module's example] shows. To run
a task on a specific thread, spawn it on that thread's runtime. A [`SharedRuntime`] can be spawned
on from any thread, as the second example above shows.

## Per-thread runtimes

The examples above run their runtime with one `block_on` call for the whole program. A library
such as [zbus] works differently: it may call `block_on` once per operation, or have its futures
polled by another executor, and its tasks and timers must keep running in between.

The `helper` feature supports this:

* The free [`block_on`] runs a future on the calling thread's own runtime.
* [`SharedRuntime::current`] returns the runtime for the calling code. From the future passed to
  the free `block_on`, or from code running on a runtime that `current` returned, it is that
  runtime. From anywhere else, code on a runtime from `SharedRuntime::new` included, it is one
  runtime that the whole process shares.
* If such a runtime has work and no thread is running `block_on` on it, a helper thread runs it
  in the meantime. A `block_on` call takes the runtime back for as long as it lasts. On WASI,
  there is no helper thread (see [WASI](#wasi)).
* [`spawn`] works from any thread. If no shared runtime runs the calling code, the task goes on
  the runtime that [`SharedRuntime::current`] returns.

## WASI

The runtime builds for WASI, `wasm32-wasip1` and `wasm32-wasip2`, where it waits on `poll(2)`.
WASI cannot start a thread, so there:

* A runtime runs only while a thread is inside `block_on` on it. With the `helper` feature, no
  helper thread runs its work in between, as [`SharedRuntime::current`] details.
* [`unblock()`] and [`Unblock`] are not built. Their work would have to run on the thread that
  polls it, which would block every task on that thread until the work is done. The documentation
  of each item says what could change that.

No `cfg` tells a WASI target with threads, such as `wasm32-wasip1-threads`, from one without, so
such a target is treated the same way. There, a wake from a thread of the program's own reaches a
runtime only once the wait the runtime is in ends.

The sockets of the [`net`] module work on `wasm32-wasip2`, where the host grants the network, as
`wasmtime run -S inherit-network` does. `wasm32-wasip1` cannot create a socket: the TCP sockets
work there only on one that the host hands in, and the UDP socket not at all. WASI has no
unix-domain sockets and no child processes, so the `unix` and `process` features build nothing
there.

The [`fs`] module works on both previews as it does elsewhere, with these differences:

* A path reaches only the directories that the host lends the program. With wasmtime,
  `wasmtime run --dir ./data::/data` lends the directory `./data` as `/data`.
* WASI has no permissions, so `fs::set_permissions` and `File::set_permissions` fail with
  `Unsupported`.
* There is no `unix` or `windows` module of extensions.

## Combinators

zruntime has no combinators of its own. Use those of the [`futures`] crate instead. They work with
zruntime's tasks, timers, sockets and pipes, which implement the `Future`, `Stream`,
`AsyncRead` and `AsyncWrite` traits.

* [`futures::future`] joins and races futures.
* [`futures::stream`] adapts streams, such as an interval or the connections of a listener.
* [`futures::io`] adds methods such as `read_to_end` and `write_all`, and a `BufReader` to read
  lines with. The [`Unblock`] documentation shows it reading the standard input line by line.

The default features of `futures` bring an executor, which you do not need with zruntime. They
also bring the `join!` and `select!` macros, which build a proc-macro crate. To leave both out:

```toml
[dependencies]
futures = { version = "0.3", default-features = false, features = ["std"] }
```

`std` brings the I/O extension traits. Add `async-await` to get the macros back.

```rust
use std::time::Duration;

use futures::future;
use zruntime::LocalRuntime;

let runtime = LocalRuntime::new().expect("a runtime for this thread");
runtime.block_on(async {
    // Two tasks after the same answer, one of them much faster than the other.
    let soon = runtime.clone();
    let near = runtime.spawn("near", async move {
        soon.sleep(Duration::from_millis(1)).await;
        "near"
    });
    let late = runtime.clone();
    let far = runtime.spawn("far", async move {
        late.sleep(Duration::from_secs(60)).await;
        "far"
    });

    // The first to answer wins. The other one is returned with the answer, and dropping it
    // cancels it.
    let (answer, _) = future::select(near, far).await.factor_first();
    assert_eq!(answer.expect("the task did not panic"), "near");

    // Two futures awaited together, keeping the outputs of both.
    let (a, b) = future::join(async { 21 }, async { 2 }).await;
    assert_eq!(a * b, 42);
});
```

To race a future against a timer, use [`Runtime::timeout`].

## Cargo features

* `runtime` (default): [`Runtime`], [`LocalRuntime`], [`SharedRuntime`], tasks, timers,
  [`AsyncIo`] and [`Registration`].
* `event` (default): [`Event`] and [`EventListener`].
* `tracing` (default): log through [`tracing`]. Without it, the runtime logs nothing.
* `helper`: [per-thread runtimes](#per-thread-runtimes). Implies `runtime`.
* `lock`: the [`lock`] module. Implies `event`.
* `mpmc`: the [`mpmc`] module. Implies `event`.
* `broadcast`: the [`broadcast`] module. Implies `event`.
* `unblock`: [`unblock()`] and [`Unblock`], except on WASI.
* `fs`: the [`fs`] module.
* `tcp`: `TcpListener` and `TcpStream` in the [`net`] module. Implies `runtime`.
* `udp`: `UdpSocket` in the [`net`] module. Implies `runtime`.
* `unix`: the `net::unix` module, on unix only. Implies `runtime`.
* `process`: the [`process`] module, except on WASI. Implies `runtime` and `unblock`.

`runtime` and `event` do not depend on each other. A crate that only needs `Event`, or the locks
and channels built on it, can turn off the default features. It then enables only what it uses,
such as `features = ["lock"]`, and builds none of the runtime.

## Why?

The project grew out of the need for a single-threaded runtime in [zbus] that it would use by
default. It was split into a separate project so non-zbus users can use it too.

## License

[MIT]

## Sponsors

<a href="https://codspeed.io/?utm_source=oss-sponsorship&utm_medium=z-galaxy">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://codspeed.io/codspeed-logo-dark.svg">
    <source media="(prefers-color-scheme: light)" srcset="https://codspeed.io/codspeed-logo-light.svg">
    <img alt="CodSpeed logo" src="https://codspeed.io/codspeed-logo-light.svg" width="400">
  </picture>
</a>

[`Runtime`]: https://docs.rs/zruntime/latest/zruntime/struct.Runtime.html
[`Runtime::block_on`]: https://docs.rs/zruntime/latest/zruntime/struct.Runtime.html#method.block_on
[`LocalRuntime`]: https://docs.rs/zruntime/latest/zruntime/type.LocalRuntime.html
[`SharedRuntime`]: https://docs.rs/zruntime/latest/zruntime/type.SharedRuntime.html
[`SharedRuntime::current`]:
    https://docs.rs/zruntime/latest/zruntime/struct.Runtime.html#method.current
[`block_on`]: https://docs.rs/zruntime/latest/zruntime/fn.block_on.html
[`unconstrained`]: https://docs.rs/zruntime/latest/zruntime/fn.unconstrained.html
[`Runtime::spawn`]: https://docs.rs/zruntime/latest/zruntime/struct.Runtime.html#method.spawn
[`spawn`]: https://docs.rs/zruntime/latest/zruntime/fn.spawn.html
[`spawn_local`]: https://docs.rs/zruntime/latest/zruntime/fn.spawn_local.html
[`Task`]: https://docs.rs/zruntime/latest/zruntime/struct.Task.html
[`Task::cancel`]: https://docs.rs/zruntime/latest/zruntime/struct.Task.html#method.cancel
[`Task::detach`]: https://docs.rs/zruntime/latest/zruntime/struct.Task.html#method.detach
[`Task::is_finished`]:
    https://docs.rs/zruntime/latest/zruntime/struct.Task.html#method.is_finished
[`Runtime::sleep`]: https://docs.rs/zruntime/latest/zruntime/struct.Runtime.html#method.sleep
[`Runtime::sleep_until`]:
    https://docs.rs/zruntime/latest/zruntime/struct.Runtime.html#method.sleep_until
[`Sleep::reset`]: https://docs.rs/zruntime/latest/zruntime/struct.Sleep.html#method.reset
[`Runtime::timeout`]: https://docs.rs/zruntime/latest/zruntime/struct.Runtime.html#method.timeout
[`Runtime::timeout_at`]:
    https://docs.rs/zruntime/latest/zruntime/struct.Runtime.html#method.timeout_at
[`Runtime::interval`]:
    https://docs.rs/zruntime/latest/zruntime/struct.Runtime.html#method.interval
[`Runtime::interval_at`]:
    https://docs.rs/zruntime/latest/zruntime/struct.Runtime.html#method.interval_at
[`Interval`]: https://docs.rs/zruntime/latest/zruntime/struct.Interval.html
[`Runtime::register`]:
    https://docs.rs/zruntime/latest/zruntime/struct.Runtime.html#method.register
[`Registration`]: https://docs.rs/zruntime/latest/zruntime/struct.Registration.html
[`Registration::poll_io`]:
    https://docs.rs/zruntime/latest/zruntime/struct.Registration.html#method.poll_io
[`AsyncIo`]: https://docs.rs/zruntime/latest/zruntime/struct.AsyncIo.html
[`AsyncIo::new`]: https://docs.rs/zruntime/latest/zruntime/struct.AsyncIo.html#method.new
[`AsyncIo::readable`]:
    https://docs.rs/zruntime/latest/zruntime/struct.AsyncIo.html#method.readable
[`AsyncIo::writable`]:
    https://docs.rs/zruntime/latest/zruntime/struct.AsyncIo.html#method.writable
[`AsyncIo::read_with`]:
    https://docs.rs/zruntime/latest/zruntime/struct.AsyncIo.html#method.read_with
[`AsyncIo::write_with`]:
    https://docs.rs/zruntime/latest/zruntime/struct.AsyncIo.html#method.write_with
[`Event`]: https://docs.rs/zruntime/latest/zruntime/struct.Event.html
[`EventListener`]: https://docs.rs/zruntime/latest/zruntime/struct.EventListener.html
[`broadcast`]: https://docs.rs/zruntime/latest/zruntime/broadcast/index.html
[`mpmc`]: https://docs.rs/zruntime/latest/zruntime/mpmc/index.html
[the module's example]:
    https://docs.rs/zruntime/latest/zruntime/mpmc/index.html#spreading-work-over-threads
[`lock`]: https://docs.rs/zruntime/latest/zruntime/lock/index.html
[`unblock()`]: https://docs.rs/zruntime/latest/zruntime/fn.unblock.html
[`Unblock`]: https://docs.rs/zruntime/latest/zruntime/struct.Unblock.html
[`fs`]: https://docs.rs/zruntime/latest/zruntime/fs/index.html
[`net`]: https://docs.rs/zruntime/latest/zruntime/net/index.html
[`process`]: https://docs.rs/zruntime/latest/zruntime/process/index.html
[`futures`]: https://docs.rs/futures
[`futures::future`]: https://docs.rs/futures/latest/futures/future/index.html
[`futures::stream`]: https://docs.rs/futures/latest/futures/stream/index.html
[`futures::io`]: https://docs.rs/futures/latest/futures/io/index.html
[`tracing`]: https://docs.rs/tracing
[zbus]: https://github.com/z-galaxy/zbus
[MIT]: https://github.com/z-galaxy/zruntime/blob/main/LICENSE
