# AGENTS.md

This file provides guidance to AI coding agents when working with code in this repository.

For contribution conventions — commit-message format, atomic commits, code layout, and
more — follow the guidelines in [`CONTRIBUTING.md`](CONTRIBUTING.md).

## Project Overview

zruntime is a simple, single-threaded Rust async runtime: an owned `Runtime<M: Mode = Local>`, a
scheduler that holds tasks and hands them out to be polled, a reactor that watches registered I/O
sources (on epoll, kqueue, `select` or `poll(2)`, whichever the platform waits best on) and keeps
timers, and `Runtime::block_on` to drive a future to
completion on the calling thread. It comes in two flavours: `Local` (the default, aliased
`LocalRuntime`), which stays on the thread it was made on and holds its state in `Rc`/`RefCell`,
and `Shared` (aliased `SharedRuntime`), which may be reached from and driven on any thread and
holds its state in `Arc`/`Mutex`. A non-default `helper` cargo feature layers per-thread shared
runtimes, a seat and a helper thread on top of `Runtime<Shared>`, for callers (such as zbus) that
drive their work with one `block_on` call per operation rather than one for the whole program. It
is a standalone crate with no dependency on any particular application; it was extracted from
zbus's built-in runtime, which now depends on it.

Two default cargo features, each usable without the other, split the crate. `runtime` is the
runtime above — `Runtime`, `LocalRuntime`, `SharedRuntime`, tasks, timers, I/O registrations
and `AsyncIo`, the async handle of any source the reactor can watch — and is what needs `rustix`
on unix and `windows-sys` on Windows, and `futures-core` and `futures-io` everywhere, for the
`Stream` impl of its interval timer and the `AsyncRead` and `AsyncWrite` impls of `AsyncIo`.
`event` is `Event` and `EventListener`, a notification that tasks wait for, which needs
no runtime and works under any executor. A crate that only notifies builds zruntime with
`default-features = false, features = ["event"]` and gets none of the runtime (zbus, whatever
runtime it runs on, adds `broadcast` and `lock` to that); one that only runs tasks leaves `event`
out. `helper` implies `runtime`, and `tracing`, also a default feature, makes the runtime log
through the `tracing` crate.

A non-default `broadcast` feature, which implies `event` and adds a `futures-core` dependency,
gives `zruntime::broadcast`: an async multi-producer multi-consumer broadcast channel, moved here
from async-broadcast. It is built on `Event`, needs no runtime and works under any executor.

A non-default `mpmc` feature, which implies `event` and adds a `futures-core` dependency, gives
`zruntime::mpmc`: an async multi-producer multi-consumer channel, bounded or unbounded, each of
whose messages one receiver gets. It is built on `Event`, needs no runtime and works under any
executor.

A non-default `lock` feature, which implies `event` and adds no dependency, gives `zruntime::lock`:
an async `Mutex` and `RwLock` whose guards may be held across an await, moved here from zbus. They
are built on `Event`, need no runtime and work under any executor. Each hands out guards that borrow
it and, through `lock_arc`, `read_arc` and `write_arc` on an `Arc` of it, guards that hold a clone
of that `Arc` instead, to be kept in a struct or moved into a spawned task. The module also gives a
`Semaphore`, a lock that up to a set number of tasks may hold at once, built the same way, with
`acquire` and `acquire_arc`, a `Barrier` that tasks wait at for each other, and a `OnceCell` that is
set once, by an initialiser that may await, and that tasks can wait for the value of.

A non-default `unblock` feature, which adds `futures-io` and `futures-core` dependencies, gives
`zruntime::unblock`: a piece of blocking work run on a pool of threads kept for it, out of the way
of the async tasks, and a future of its outcome, moved here from zbus. It also gives
`zruntime::Unblock`, as smol has `smol::Unblock`: an adapter that gives a blocking I/O handle (a
file, the standard input, an iterator) `futures-io`'s `AsyncRead`, `AsyncWrite` and `AsyncSeek`
and `futures-core`'s `Stream` by running each operation on it as such work. Both need no runtime
and work under any executor. Neither is built on WASI, which cannot start a thread for the work.

A non-default `fs` feature, which adds `futures-io` and `futures-core` dependencies, gives
`zruntime::fs`: async access to the filesystem in the shape of `std::fs`, as smol has in `smol::fs`
(the async-fs crate), with each operation run on the thread that polls its future, within that poll,
and a `File` that holds std's file. No poller can say when a regular file is ready (epoll refuses to
watch one, `poll(2)` reports it as always ready, kqueue as ready while its position is short of its
end, and Winsock's `select` watches only sockets), so there is nothing to wait for: an operation
that misses the OS cache, runs on a slow or network mount or moves a large buffer blocks the thread,
and every task on it, while it runs. The module's documentation says so, and points to `unblock` for
work that must not block. Each operation spends the cooperative budget (see "Cooperative budget").
The module needs no runtime and works under any executor. It builds on WASI too, with a few
differences that the WASI paragraph below lists.

A non-default `tcp` feature, which implies `runtime` and adds the `socket2` dependency, gives the
`zruntime::net` module's TCP sockets, `TcpListener` and `TcpStream`, as smol has in `smol::net`.
They run on a `Runtime` of either flavour, connect without blocking the thread, take socket
addresses rather than host names, and implement `futures-io`'s `AsyncRead` and `AsyncWrite`. Their
non-blocking connect is carried over from zbus, whose own socket layer stays there: it drives its
sockets through whichever runtime a connection runs on. A non-default `udp` feature, which implies
`runtime` and adds no dependency, gives the module's `UdpSocket`, likewise. A non-default `unix`
feature, which implies `runtime` and adds the `socket2` dependency and `rustix`'s `net` feature,
gives the `zruntime::net::unix` module's `UnixListener`, `UnixStream` and `UnixDatagram`, on unix
only: on Windows and WASI it builds nothing.

A non-default `process` feature, which implies `runtime` and `unblock` and adds `rustix`'s
`process` feature on unix and `windows-sys`'s `Win32_Foundation` and `Win32_System_Threading`
features on Windows, gives `zruntime::process`: async child processes in the shape of
`std::process` and of smol's `smol::process` (the async-process crate), a `Command`, a `Child` and
the pipes to a child, `ChildStdin`, `ChildStdout` and `ChildStderr`, which implement `futures-io`'s
`AsyncWrite` and `AsyncRead`. They run on a `Runtime` of either flavour, which a child is spawned
on as an argument. On unix a child's pipes are watched by the runtime's reactor; on Windows they go
through `Unblock`. Waiting for a child to exit is watched by the runtime's reactor where it can,
which it can through a pidfd on Linux and through a kqueue of the child's own on Apple's platforms
and the BSDs, which have no wait on the pool to fall back on; elsewhere, on Android, on Windows, on
the other unix systems, and on a Linux with no pidfd to give (before 5.3, or in a sandbox that turns
the call away), it runs on a thread of a pool of its own (`WAITS` in `src/process/mod.rs`, its
threads named `zruntime child wait`, with the cap and the idle timeout of `unblock`'s pool),
through a `waitid` that leaves the child unreaped on unix. Either way only std's `Child` ever
reaps the child, through `try_wait` while the `Child` is held and through `try_wait` and `wait`
once it is dropped, so that a `kill` never reaches a reused process ID. Dropping a `Child` leaves
the process running unless `kill_on_drop(true)`; on unix, a still-running child that is let go of
is reaped from a thread of that same pool unless `reap_on_drop(false)`, so that the waits of a
program that lets go of, or waits for, many long-running children never fill the pool that the
rest of the blocking work shares. It builds nothing on WASI, which has no child
processes.

The crate builds for WASI, `wasm32-wasip1` and `wasm32-wasip2`, too. There the runtime waits on
`poll(2)` (`src/poll/generic.rs`), whose `notify` raises a flag that ends the next wait rather than
writing to a pipe, which WASI lacks, and bounds a wait with no limit by `MAX_TIMEOUT`, which WASI
would otherwise fail (preview 1) or trap on (preview 2) where nothing is watched. WASI cannot start
a thread, so nothing in the crate starts one there: `driver::spawn_helper` starts no helper, so a
runtime with a seat runs only inside `block_on` on it. It checks `cfg!(target_os = "wasi")`, so that
the threads' side of the code still builds there; no `cfg` tells `wasm32-wasip1-threads` apart, so
it gets the same treatment. Blocking work would have to run on the thread that polls it, blocking
every task on that thread, so the `unblock` feature builds nothing on WASI: each of its public items
has a `WASI` section in its documentation, from the `not_on_wasi!` macro of its module, that says
why and what could change it. The `fs` feature builds there, as it needs no thread of its own (it
runs each operation on the thread that polls it everywhere), and works on both previews, with these
differences: a path reaches only the directories the host lends the program (`wasmtime run --dir`);
`set_permissions` and `File::set_permissions` fail with `Unsupported`, WASI having no permissions;
there is no `unix` or `windows` module; and `File` has its `AsFd`, `AsRawFd` and `From<OwnedFd>`
impls there as on unix. `std::thread::park` returns at once on WASI, so nothing reachable there may
wait in a park for another thread. The `tcp` and `udp` sockets work on preview 2; preview 1 cannot
create a socket, so socket2, which does not build for it, is no dependency there, and the
constructors that create one fail with `Unsupported` (a TCP socket's `from_std` takes one the host
handed in). Preview 1 has no UDP at all: std fails every operation of its UDP socket there, so
`UdpSocket::from_std` fails too. The `unix` and `process` features build nothing on WASI. `ci/wasi`
is a crate of its own that runs the runtime, the shared runtimes, the `fs` module and the sockets on
both previews under wasmtime, with a host directory lent to it as `/data` for the files of the `fs`
module.

It is a single crate at the repository root — not a workspace.

## Common Development Commands

### Building and Testing
```bash
# Full test suite, every feature on (the features all but only add code: what they take out is
# the no-op `error!` in `log.rs` and what `spawn` does without `helper`, which the next command
# tests)
cargo test --all-features

# The runtime's tests without `event` and `helper`, the one of `spawn` without `helper` among them
cargo test --no-default-features --features runtime

# Run a single test
cargo test --all-features some_test_name

# The tests on the pollers no CI runner waits on by default, as CI runs them: `poll(2)` on any unix
# (on Linux, say), and kqueue on macOS, where the BSDs' poller runs as well
RUSTFLAGS="--cfg zruntime_poll" cargo test --all-features
RUSTFLAGS="--cfg zruntime_kqueue" cargo test --all-features
```

### Code Quality
```bash
# Format code (requires nightly)
cargo +nightly fmt --all

# Lint with clippy
cargo clippy --all-targets --all-features -- -D warnings

# Lint the pollers no CI runner waits on by default, as CI does: `poll(2)` on the host and on
# each unix target (`--target x86_64-unknown-freebsd`, say), and kqueue on macOS
RUSTFLAGS="--cfg zruntime_poll" cargo clippy --all-targets --all-features -- -D warnings
RUSTFLAGS="--cfg zruntime_kqueue" cargo clippy --all-targets --all-features \
    --target x86_64-apple-darwin -- -D warnings

# Lint the locks and channels for `wasm32-unknown-unknown`, which the wasm check below runs the
# locks on, as CI does
cargo clippy --no-default-features --features lock,broadcast,mpmc \
    --target wasm32-unknown-unknown -- -D warnings

# Check the runtime, Event, the two channels, the locks, unblock, fs, each family of socket and
# the child processes built alone: `--all-features` cannot show that each builds without the
# others, and leaves out the no-op `error!` in `log.rs` that replaces `tracing`'s
cargo check --no-default-features --features runtime
cargo check --no-default-features --features event
cargo check --no-default-features --features broadcast
cargo check --no-default-features --features mpmc
cargo check --no-default-features --features lock
cargo check --no-default-features --features unblock
cargo check --no-default-features --features fs
cargo check --no-default-features --features tcp
cargo check --no-default-features --features udp
cargo check --no-default-features --features unix
cargo check --no-default-features --features process

# Run what needs no runtime (Event, the locks, the two channels, unblock) under Miri, as CI does;
# the runtime's poller on Linux keeps a timerfd, which Miri does not run, and fs reaches the
# filesystem, which Miri's isolation keeps it from
cargo +nightly miri test --no-default-features --features lock,broadcast,mpmc,unblock

# Run the locks' waiting paths on `wasm32-unknown-unknown`, which has no clock, in Node, as CI
# does
cargo build --release --target wasm32-unknown-unknown --manifest-path ci/wasm/Cargo.toml
node ci/wasm/run.mjs ci/wasm/target/wasm32-unknown-unknown/release/zruntime_wasm_check.wasm

# Lint the builds for WASI, as CI does: with every feature on, and with each alone
cargo clippy --all-features --target wasm32-wasip1 -- -D warnings
cargo clippy --all-features --target wasm32-wasip2 -- -D warnings
cargo clippy --no-default-features --features tcp --target wasm32-wasip1 -- -D warnings

# Run the runtime, the shared runtimes, the filesystem module and the sockets on WASI under
# wasmtime, as CI does: the filesystem module works in a host directory that `--dir` lends as
# `/data`, and the sockets need preview 2 and the network
cargo build --target wasm32-wasip1 --manifest-path ci/wasi/Cargo.toml
cargo build --target wasm32-wasip2 --manifest-path ci/wasi/Cargo.toml
data=$(mktemp -d)
wasmtime run --dir "$data"::/data ci/wasi/target/wasm32-wasip1/debug/zruntime-wasi-check.wasm
wasmtime run -S inherit-network --dir "$data"::/data \
    ci/wasi/target/wasm32-wasip2/debug/zruntime-wasi-check.wasm

# Check cross-platform compatibility
cargo check --all-features --target x86_64-pc-windows-gnu
cargo check --all-features --target x86_64-apple-darwin
cargo check --all-features --target x86_64-unknown-freebsd
cargo check --all-features --target x86_64-unknown-netbsd
cargo check --all-features --target aarch64-linux-android
```

### Documentation
```bash
cargo doc --all-features
```

### Benchmarks
```bash
# Run benchmarks (the runtime, spawn and connection ones need the helper feature: they measure the
# block_on-per-operation case; the event and connection ones need the lock feature, which implies
# Event)
cargo bench --features helper,lock

# The broadcast channel's benchmark needs the broadcast feature and no runtime
cargo bench --features broadcast --bench broadcast

# The MPMC channel's benchmark needs the mpmc feature and no runtime
cargo bench --features mpmc --bench mpmc

# The channel and the mutex between contending threads, and the channel between the tasks of a
# runtime, need the mpmc and lock features and the default runtime one
cargo bench --features mpmc,lock --bench contention

# The benchmark of handing work to `unblock` needs the unblock feature and no runtime
cargo bench --features unblock --bench unblock
```

On CodSpeed, `.github/workflows/bench.yml` measures each bench target in one of two jobs: by CPU
simulation, on a GitHub-hosted runner, the targets that run on one thread, with no runtime; by the
clock, on a CodSpeed macro runner, those that wait on timers, sockets or other threads, or run on a
runtime, which waits on its reactor at every turn. A target holds benchmarks of one kind only, and
a new one goes in the list of its job.

## Architecture Overview

```
src/
├── lib.rs        # Public API: Runtime, LocalRuntime, SharedRuntime, Registration, Readiness,
│                 # Interest, AsyncIo, Source, Task, Sleep, Timeout, TimedOut, Interval,
│                 # MissedTickBehavior, and the free spawn, spawn_local and unconstrained
│                 # (runtime feature), Event, EventListener (event feature), and the free
│                 # block_on (helper feature)
├── async_io.rs   # [runtime feature] AsyncIo<T, M>: the async handle of any source the reactor
│                 # can watch
├── event.rs      # [event feature] Event/EventListener: a notification tasks wait for, under
│                 # any executor, with no use of the runtime
├── event-only.md # The crate's documentation in a build with `event` but not `runtime`, whose
│                 # README examples it cannot run
├── broadcast.rs  # [broadcast feature] The async multi-producer multi-consumer broadcast channel,
│                 # built on Event, with no use of the runtime
├── mpmc.rs       # [mpmc feature] The async multi-producer multi-consumer channel, each
│                 # message to one receiver, built on Event, with no use of the runtime
├── fs/           # [fs feature] Async filesystem access, each operation run within the poll of
│                 # its future, with no use of the runtime
├── lock/         # [lock feature] Mutex, RwLock, Semaphore, Barrier and OnceCell, built on Event,
│                 # with no use of the runtime
├── log.rs        # [runtime feature] Logging through `tracing`, or nothing without it
├── mode.rs       # [runtime feature] The sealed `Mode` trait: what Local/Shared build state from
├── net/          # [tcp, udp, unix features] Async sockets, each built on `AsyncIo<T, M>`:
│                 # connect.rs, the non-blocking connect; tcp.rs; udp.rs; unix.rs, the
│                 # `net::unix` module (unix only)
├── process/      # [process feature, not on WASI] Async child processes: mod.rs, Command and
│                 # Child, and the pool that the waits for children run on; stdio.rs, the pipes;
│                 # exit/, the wait for a child's exit: kqueue.rs (Apple, the BSDs), pool.rs
│                 # (elsewhere, behind a pidfd on Linux)
├── runtime.rs    # [runtime feature] Core<M>: scheduler + reactor + driving state of a Runtime<M>
├── scheduler.rs  # [runtime feature] Holds spawned tasks and hands them out to be polled
├── reactor.rs    # [runtime feature] Watches registered I/O sources and keeps timers
├── coop.rs       # [runtime, fs features] The budget of I/O operations one poll may complete
│                 # before the next yields, kept in a thread-local, and unconstrained, which lifts
│                 # it; fs spends it with no runtime
├── time.rs       # [runtime feature] The timers a runtime hands out: Sleep, Timeout and Interval
├── poll/         # [runtime feature] The OS pollers: epoll.rs (Linux, Android), kqueue.rs (the
│                 # BSDs), select.rs (Apple), generic.rs (poll(2), any other unix, and WASI),
│                 # windows.rs (Winsock's select); list.rs, the list the last three keep;
│                 # pipe.rs, the channel that breaks a unix wait
├── driver.rs     # [helper feature] the seat/helper-thread machinery, per-thread registries
├── unblock/      # [unblock feature] Blocking work on a pool of threads (pool.rs), and the
│                 # Unblock adapter of a blocking I/O handle (io.rs), with no use of the runtime
└── tests/        # core.rs: Local + Shared, runtime feature; coop.rs: the I/O budget, runtime
                  # feature; event.rs: Event, event feature; broadcast.rs: the broadcast channel,
                  # broadcast feature; mpmc.rs: the MPMC channel, mpmc feature; lock/: the locks,
                  # lock feature; helper.rs: helper feature; unblock/: unblock, its pool and
                  # Unblock, unblock feature; fs.rs: fs, fs feature; net/: the sockets, each
                  # family under its own feature; process.rs: child processes, process feature
```

The tests in `src/tests/` are the unit tests of the crate, one binary whose tests run side by side.
`tests/process.rs`, at the repository root, is an integration test and a binary of its own
[process feature]: its one test holds every thread of the pool that all blocking work shares, which
would stall any other test that ran beside it, so no other test goes in that file.

### Key Design Patterns

**Two flavours over a sealed `Mode`**: `Runtime<M: Mode = Local>` is generic over how it shares
its state. `Local` builds it from `Rc`/`RefCell`/`Cell`, stays on its thread, and runs any
`'static` future; `Shared` builds it from `Arc`/`Mutex`, is `Send + Sync` by auto traits alone (the
runtime has no `unsafe impl`: the crate's only ones are the `lock` module's `Send` and `Sync`
impls), and runs `Send` futures. The sealed trait in `mode.rs` is the single place that names which:
everything else in `scheduler.rs`, `reactor.rs` and `runtime.rs` is written once, generic over `M`.

**The wake path is the only cross-thread part of a `Local` runtime**: a `Waker` must be
`Send + Sync` even for a `Local` task, whose future is not, so a task's waker never holds the
future — it holds an id into the scheduler's task map plus an `Arc<Remote>` (an atomic-backed
ready queue and the poller's notify half). This is the one place a `Local` runtime pays for
atomics; everywhere else it is a plain `Rc`/`RefCell`/`Cell` structure. On a `Shared` runtime the
waker lives inside the task's join state instead, saving an allocation per task.

**Single-threaded, seat-based driving (the `helper` feature)**: on top of `Runtime<Shared>`,
`driver.rs` gives a runtime made through the per-thread registries (`SharedRuntime::current()`,
the free `block_on`) a seat: only one thread at a time runs its scheduler and reactor. A thread
that calls `block_on` takes the seat for as long as the call lasts; when no thread is running
`block_on` on the runtime, a helper thread takes the seat instead, so that spawned tasks, timers
and registered I/O still make progress. The helper parks as soon as a thread calls `block_on` to
take the seat back, and puts itself down once it finds nothing left to run, watch or time. A
`Runtime::new()` runtime never has a seat or a helper: it runs only while some thread is running
`block_on` on it. On WASI no helper is ever started, so a runtime with a seat runs that way too.

**Parallelism through several runtimes**: a runtime's scheduler and reactor are only ever run by
one thread at a time, so all of its tasks share one core, and that stays so: several threads
taking tasks from one runtime's scheduler, as smol's `Executor::run` does, was decided against.
Work that needs more cores runs on several runtimes, one per thread, spread over them through an
`mpmc` channel whose receiver each thread holds a clone of. The README's "Running on several
threads" section and the `mpmc` module's "Spreading work over threads" example document this.

**I/O integration**: `Runtime::register` erases the source into the mode's `SourcePtr` (`Rc<dyn
AsFd>` / `Arc<dyn AsFd + Send + Sync>` on unix, `AsSocket` on Windows) and returns a
`Registration` whose `poll_io` drives an arbitrary operation against
`Interest::Readable`/`Writable` readiness, retrying on `WouldBlock`, and whose `ready` hands out a
`Readiness` future that waits for readiness alone. The source's descriptor is read once, at
registration, and kept beside it in the map: the poller watches that and calls no `as_fd` or
`as_socket`, so it runs no code of a source's. A runtime watches a descriptor through one
registration at a time, and turns a second away with `AlreadyExists`. Each direction of a source
keeps one waker for `poll_io`, which the next operation to wait takes the place of, and a map of
`Readiness` waits under ids of their own, any number of which may wait at once: readiness wakes
them all and takes them out of the map, which is how a `Readiness` tells readiness from a poll
anything else caused.

**Telling the poller what changes**: the reactor tells the poller (`src/poll/`) of each change to
what a source is watched for, under the map's lock, rather than handing it every source on each
wait: `add` as a source is registered, `modify` as the directions it is watched in change, and
`delete` as its registration goes. A direction is armed as soon as a waiter is stored in it, and
disarmed lazily: a source a wait found ready, or a `Readiness` gave up on, is listed in
`Sources::stale` and looked at before the next wait, so that a task that stores its waker again
before then costs the poller nothing. `Poller::LIVE` says whether a change reaches a wait under
way (a poller that keeps what it watches in the kernel) or only the next one (one that keeps a
`poll/list.rs` list and copies it as a wait starts). For the latter, arming breaks the wait under
way, and a source whose registration goes while a wait runs is kept in `Sources::retired`, so
that its descriptor stays open, until that wait returns.

**`AsyncIo`, the handle of a source**: `AsyncIo<T, M>` is a `Registration` and the mode's `Ptr<T>`
(`Rc`/`Arc`) of the source, of which the reactor holds a clone through the sealed
`IntoSource::source_ptr`; its I/O runs on the `Ptr<T>`. The constructors are written once, generic
over `M`, with `T: Source<M>`: a sealed public bound, whose per-flavour blanket impls go through the
private supertrait `IntoSource<M>` (`AsFd`/`AsSocket` and `'static`, and `Send + Sync` for
`Shared`). A `new` in an `impl` of each flavour instead would make `AsyncIo::new` ambiguous (E0034),
as the compiler looks the item up before it has chosen the flavour. No `&mut T` is handed out, the
reactor sharing the pointer, so the I/O traits are there only where `&T` implements `Read`/`Write`,
for `AsyncIo` and `&AsyncIo` both. `readable`, `writable`, `read_with` and `write_with` wait through
`Readiness`, any number of tasks at once; `poll_read_with`, `poll_write_with` and the traits keep
one waiting task per direction, through `poll_io`. `into_inner` ends the watch and takes the source
out of the pointer: on `Local` nothing else holds it by then, and on `Shared` it yields until a wait
under way on another thread returns and the reactor lets go of the source it kept for that wait,
which it does clear of every lock and before it wakes anyone.

**Cooperative budget**: a poll runs until it returns, so a task whose I/O is always ready would hold
the thread, and with it the other tasks and the reactor, for as long as its loop lasts. `coop.rs`
limits that with a budget of `BUDGET` (128) I/O operations per poll, kept in the `LEFT`
thread-local. `coop::budgeted` sets it for the length of one poll: in `Scheduler::run`, around a
task's poll (which covers `run_batch` from `block_on`, from the seat and from the helper alike), and
in `Core::block_on` and `driver::block_on`, around the poll of the future passed to them, which the
tasks wait out as they wait out each other. Four places spend it, and they are the only four that
run an I/O operation: `Registration::poll_io`, `AsyncIo::run_with` (the body of `read_with` and
`write_with`), `UnixDatagram::send_to`, which waits for room on a timer rather than for readiness
(see `SEND_TO_INTERVAL`), so that neither of the first two can run its send, and the `fs` module,
whose files no reactor can wait on, in `fs::inline` (the future of each function or method that
makes a single call), `poll_inline` in `fs/file.rs` (a read, a write or a seek of a `File`) and
`ReadDir::poll_next`. Each calls `poll_proceed` before the operation, which, once the budget is
zero, wakes the task with `wake_by_ref` and returns `Pending` without running it, every time it is
asked until the poll is over, and `spend` once the operation has completed. For the first three that
is anything but `WouldBlock`, which they wait out on the reactor or, for `send_to`, on a timer; the
`fs` places have nothing to wait on, so they spend on every result, `WouldBlock` included.
`run_with`, `send_to` and `poll_inline` retry an `Interrupted` call straight away, and that spends
nothing; `poll_io` hands it back like any other result. The other sockets, the traits of `AsyncIo`
and, on unix, the pipes to a child go through the first two. `readable`, `writable` and
`Registration::ready` spend nothing, nor do the waits for a child's exit that use them, nor do the
pipes on Windows, which are an `Unblock`. A yield puts the task at the back of the ready queue,
behind the tasks queued before it, but the reactor is waited on only at the end of a batch
(`Core::round`), and `Scheduler::run_batch` takes a task its own polls woke again within the same
batch: a task alone in the queue runs up to `BATCH` times `BUDGET` operations before the reactor has
its turn.

The budget is per thread, not per task: a poll runs on one thread from its start to its end, so an
operation finds it without being handed it, and a thread inside no poll by a runtime of this crate
has `None`, for which `poll_proceed` always proceeds and `spend` does nothing, so that another
executor is not held to a budget outside such a poll. Inside one it is: an executor of another crate
that a task runs, such as `futures::executor::block_on`, never calls `budgeted`, so it shares what
is left of the task's budget. Once that is gone, an executor that blocks until its future is done
polls again at once, inside the same poll of the task, is turned away again, and spins for good. So
it has to run its future inside `unconstrained`, public and re-exported at the crate root (tokio has
the same as `tokio::task::unconstrained`): each poll it makes of its future sets `LEFT` to `None`
and puts the old value back through `Restore`, so that the operations of that future neither wait on
the budget nor spend it, and the poll around it goes on with what it had left. It is an `async fn`,
where tokio's returns a type of its own, `Unconstrained`: such a type would have to pin the future
it runs structurally, which takes `unsafe` code, a dependency such as `pin-project-lite`, or a box
on every call. No rule in `poll_proceed` can let such an executor through by itself, because a waker
does not identify a future. `select_all` polls many leaf futures with one waker, the task's, within
one poll; a blocking executor polls one leaf again and again with one waker, its own, within one
poll; and `poll_proceed` sees the same in both: operations that ask inside one poll, one after
another, each with a waker that the refusal before it woke. A cap on how many operations a poll may
turn away, or any other heuristic on what `poll_proceed` sees, lets the operations of the combinator
through along with those of the executor: with such a cap at `BUDGET`, a task that has `select_all`
poll `BUDGET + 1` readers of a source that is always ready would read as much in one poll as nine
budgets allow, and `a_task_that_selects_over_more_readers_than_its_budget_yields_to_the_next` in
`src/tests/async_io.rs` would fail. Keying the budget on the task's waker fails the same way: a
combinator that wraps the waker (`FuturesUnordered`, `Shared`) would have its futures let through.
`LEFT` is a `Cell<Option<u8>>` with a constant initialiser, so it has no destructor, like the
`driven` record and the `Remote` marker (see "Spawning without a runtime handle"): a `block_on` run
from the destructor of another thread-local can still reach it, and is budgeted like any other.
`budgeted` puts the old value back, through the drop guard `Restore`, instead of clearing it, so
that a poll made from inside another that is budgeted too ends with what was left of the poll it
came from. The one case of that is the free `block_on` called from inside the future of another call
of it that holds no seat (`leave_the_outer_call`): a `Runtime::block_on` called from a task, or from
inside the future of any `block_on`, panics. It puts the old value back as a panic unwinds too: the
scheduler's poll sits inside `catch_unwind`, and a caller can catch a panic out of `block_on`, and a
budget left behind by either would belong to no poll, and would turn away the operations of the
plain code after it for good, though it has no tasks to give the thread to. The number is named in
the documentation of `Registration::poll_io` and of the `fs` module, in the README's Tasks and Files
sections and here, so a change to `BUDGET` is a change in each.

`coop.rs` is also built for `fs` without the runtime (`any(feature = "runtime", feature = "fs")`).
That does not break "must not reach into the runtime": the module is a thread-local and four
functions (`budgeted`, `unconstrained`, `poll_proceed` and `spend`), and names no type of the
runtime, its scheduler or its reactor. `poll_proceed` and `spend`, the two that `fs` calls, are
built for either feature; `budgeted`, `unconstrained`, `Restore` and `BUDGET`, which set a budget or
lift it, for `runtime` alone. With no runtime nothing sets `LEFT`, so it is `None` on every thread,
`poll_proceed` always proceeds and `spend` does nothing: every `fs` operation runs, whatever
executor polls it.

**Cooperative cancellation**: dropping a `Task` cancels it; `Task::detach` lets it run to
completion unobserved; `Task::is_finished` tells, without polling it, whether it has ended.
`Task::cancel` cancels it and hands back a future that resolves once the task's future is gone:
to its output, where the task had finished first. A cancelling handle stops the task as a drop
does, but leaves the outcome for the guard in the task's wrapper to settle, which it does only
once the future has been dropped.

**Spawning without a runtime handle**: the free `spawn` and `spawn_local` find their runtime in a
per-thread record of the runtime of their flavour that the thread drives (`set_driven` and
`driven` of the sealed trait, a `Weak` in a thread-local of each impl), which `set_driving` in
`runtime.rs` writes together with the `NonNull<Remote>` marker that wakes read, for `block_on` on
a runtime with no seat and for the seat of one with a seat alike. Like the marker, the record has
no destructor (its `Weak` is in a `ManuallyDrop`, and every writer clears it as the thread stops
driving), so that a `block_on` run from a thread-local's destructor finds it. `spawn` falls back,
with `helper`, on what `SharedRuntime::current` resolves to, which differs from the record for a
`Runtime::new` runtime being driven: `driver::driven` leaves a runtime with no seat out. A runtime
that fallback brings into being while the thread is running the free `block_on` is held by that
call from then on (`driver::current_to_spawn_on`), as nothing else would hold it, in a thread-local
of its own (`HELD`) that, like the records, has no destructor. Both name the task by
`Location::caller()` through `scheduler::Name`, which never allocates.

**Sockets (the `net` features)**: every socket is an `AsyncIo<T, M>`: the std socket, non-blocking,
in the mode's `Ptr` (`Rc`/`Arc`), of which the reactor holds a clone through the sealed
`Mode::source_ptr`, and the `Registration` its I/O waits on, made by the crate-private
`AsyncIo::from_nonblocking`. The async methods (`accept`, `peek`, `recv`, `send` and the like) wait
through `read_with` and `write_with`, which wait on `Registration::ready`, so any number of tasks
wait in them at once. The poll-based traits and the `Incoming` streams go through `poll_read_with`
and `poll_write_with` instead, and so through `Registration::poll_io`, which keeps one waker per
direction: a socket serves one waiting reader and one waiting writer at a time through them, and
neither kind of wait takes the place of the other. A stream implements `futures-io`'s traits for
`&Stream` as well, which is how a reader and a writer share it. No socket is `Clone`.

## Development Guidelines

- **MSRV**: 1.88.0
- **Commit style**: Emoji prefix only, no package prefix — this is a single crate (e.g.,
  "🐛 Fix timer rounding").
- **Changelog**: `CHANGELOG.md` is managed by [release-plz] — do **not** hand-edit it. Write a
  good commit message (conventional-commits-ish) and release-plz will generate the entry at
  release time.
- **Features**: `runtime` and `event` each build without the other. Nothing in `event.rs` may reach
  into the runtime, and a test of the runtime that uses an `Event` is gated with
  `#[cfg(feature = "event")]` (`cargo test --no-default-features --features runtime` builds the
  tests without it). `broadcast`, `mpmc` and `lock` imply `event` and, like it, must not reach into
  the runtime; nor may `unblock` or `fs`, which need neither (`fs` spends the budget kept by `coop`,
  which is built without the runtime when only `fs` is on: see "Cooperative budget"). `unblock`
  builds nothing on WASI, and `fs` builds there. `tcp`, `udp` and `unix` imply `runtime`, and `unix`
  builds nothing on Windows or WASI. `process` implies `runtime` and `unblock`, and builds nothing
  on WASI. CI builds and tests everything with every feature on, tests `runtime` alone (for what
  `spawn` does without `helper`), and checks `runtime`, `event`, `broadcast`, `mpmc`, `lock`,
  `unblock`, `fs`, `tcp`, `udp`, `unix` and `process` each alone, on the host and on both WASI
  targets.
- **Testing**: The test suite needs no external services (no D-Bus, no network beyond loopback).
  Tests, doc examples and benchmarks take `block_on`, the combinators and the I/O extension traits
  from the `futures` dev-dependency (no default features; `std` and `executor`), the crate the
  README points users at, and not from smol's futures-lite. `src/tests/mod.rs` has the
  `yield_now` and `or` that `futures` lacks.
- **Cross-platform**: Validate changes work on Linux, Windows, macOS (and ideally the BSDs and
  Android, which CI also checks).
- **Dependencies**: Keep this crate's own dependency footprint small; it is meant to be pulled in
  by other crates (such as zbus) that want an async runtime with no external deps of their own.

[release-plz]: https://release-plz.ieni.dev/

## Key Files for Understanding

- `src/lib.rs`: Public API and the `Runtime` handle
- `src/mode.rs`: The sealed `Mode` trait behind `Local`/`Shared`
- `src/runtime.rs`: `Core<M>`, the scheduler + reactor + driving state `Runtime<M>` owns
- `src/driver.rs`: [helper feature] the free `block_on` and the seat/helper-thread machinery
- `src/reactor.rs`: I/O readiness and timers
- `src/async_io.rs`: `AsyncIo<T, M>`, the async handle of any source the reactor can watch, built on
  a `Registration` and the `Ptr<T>` of the source it shares with the reactor
- `src/scheduler.rs`: Task storage and polling
- `src/coop.rs`: [runtime, fs features] The budget of I/O operations one poll may complete before
  the next yields, which the scheduler and the `block_on`s set, `poll_io`, `AsyncIo` and the `fs`
  module spend, and `unconstrained` lifts
- `src/time.rs`: The timers a runtime hands out, each built on a deadline its reactor keeps
- `src/event.rs`: `Event`/`EventListener`, a queue of listeners in a slab behind one mutex, and an
  atomic word beside it that lets a notification that would reach nobody skip the mutex
- `src/lock/`: [lock feature] `Mutex`, `RwLock` and `Semaphore`, the async locks built on `Event`,
  with guards that borrow the lock and guards that hold an `Arc` of it, and `Barrier` and
  `OnceCell`, built on `Event` too
- `src/unblock/`: [unblock feature] `unblock`, blocking work on a pool of threads (`pool.rs`), and
  `Unblock`, the async adapter of a blocking I/O handle built on it (`io.rs`)
- `src/fs/`: [fs feature] Async filesystem access: the free functions, and `inline`, which runs
  each of them within the poll of its future and spends the budget (`mod.rs`), `File` on std's
  file (`file.rs`), `ReadDir`/`DirEntry`/`DirBuilder` (`dir.rs`), `OpenOptions` (`options.rs`), and
  the platform extension traits (`unix.rs`, `windows.rs`)
- `src/net/connect.rs`: [tcp, unix features] The non-blocking connect, and Winsock's check of one
- `src/process/`: [process feature] Async child processes: `Command` and `Child`, and the `WAITS`
  pool that the waits for children run on (`mod.rs`), the pipes to a child (`stdio.rs`), and the
  wait for a child's exit (`exit/`), which the reactor watches through a kqueue of the child's own
  on Apple's platforms and the BSDs (`kqueue.rs`) and a pidfd on Linux, and a thread of the `WAITS`
  pool runs elsewhere (`pool.rs`)
