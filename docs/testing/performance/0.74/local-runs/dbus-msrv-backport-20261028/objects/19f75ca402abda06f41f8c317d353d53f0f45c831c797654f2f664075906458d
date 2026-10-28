# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.2.0 - 2026-10-14

### ✨ Added
- Build the fs module on WASI.
- Build the TCP and UDP sockets on WASI.
- Run the reactor on WASI.

### 💥 Breaking
- Have the fs feature no longer turn on unblock and lock.

### 👷 CI
- Run the fs module on WASI in CI.
- Lint and run zruntime on WASI in CI.

### ♻️ Changed
- Run the operations of fs on the thread that polls them.
- Yield from a poll after 128 I/O operations.
- Build nothing for the process feature on WASI.
- Start no helper thread on WASI.
- Build no blocking work on WASI.

### 📝 Documentation
- Show the belt drive logo in the README and API docs.
- Replace the Shuttle logo with a belt drive.
- Document fs on the thread that polls it, and on WASI.
- Document the cooperative budget.
- Have docs.rs build the documentation for WASI too.
- Document the support of WASI.

## 0.1.0 - 2026-10-13

### ✨ Added
- Add UnixStream::connect_addr.
- Add TcpStream::into_std and UnixStream::into_std.
- Add Command::get_kill_on_drop and Command::get_reap_on_drop. #36
- Add async child processes. #23
- Add Async, the async handle of any fd or socket. #21
- Add zruntime::spawn and spawn_local, which need no runtime handle.
- Add Task::cancel, which waits for the task to stop.
- Add Task::is_finished.
- Add Runtime::interval, a Stream that ticks every period.
- Add Runtime::timeout, a deadline around any future.
- Let a Sleep be reset in place.
- Add Runtime::sleep_until, a sleep to a deadline.
- Warn about a Sleep that is never awaited.
- Add an async fs module.
- Add Unblock, an async adapter for blocking I/O handles.
- Add an async multi-producer multi-consumer channel.
- Add async unix-domain sockets.
- Add an async UDP socket.
- Add async TCP sockets.
- Add unblock, moved from zbus.
- Add an async Mutex and RwLock, moved from zbus.
- Add a broadcast channel, moved from async-broadcast.
- Add Event, a notification that tasks can wait for.
- Add the helper feature: per-thread shared runtimes.
- Add the runtime.
- Begin the project.

### 👷 CI
- Lint poll(2) on every unix target, and the locks on wasm32.
- Measure the benchmarks that run on a runtime by the clock.
- Run the tests on poll(2) and kqueue in CI.
- Measure the single-thread benchmarks by CPU simulation.
- Run the locks' waiting paths on wasm32-unknown-unknown in CI.
- Run the tests that need no runtime under Miri.
- Build, lint and test with every feature on.
- CI: Build, lint and test the helper feature.
- Bring in lint-related cfgs from zbus.
- Authenticate CodSpeed uploads through OIDC and add its badge.
- Run the benchmarks on CodSpeed.

### ♻️ Changed
- Rename `Async` to `AsyncIo`.
- Share the reaps' pool with the waits for a child's exit.
- Reap dropped children on a pool of their own. #35
- Give each pool of blocking work a name for its threads.
- Tell the poller what changes rather than every source each wait.
- Build the net sockets on Async, so any number of tasks may wait.
- Add Registration::ready, a wait for readiness alone.
- Add lock::OnceCell.
- Add lock::Barrier.
- Add lock::Semaphore, to cap how many tasks do something at once.
- Add Arc-owned guards to lock::Mutex and lock::RwLock.
- Move the lock tests into a directory of their own.
- Move Sleep into a module of the runtime's timers.
- Keep the benchmarks that need a clock apart from the rest.
- Share the threads of the threaded benchmarks.
- Rename unblock's future to BlockingWork.
- Let a reader that waited long in ahead of waiting writers.
- Bound how long lock::RwLock lets new writers past a writer.
- Bound how long lock::Mutex lets newcomers past a waiter.
- Put the runtime and Event behind cargo features.
- Make the runtime an owned value, local or shared.
- Crate-level docs to come from README.

### 📦 Dependencies
- Take tracing without its attribute macros, and so without syn.
- Drop futures-lite for futures in the tests, examples and benches.
- Bump the MSRV to 1.88.
- Test with zruntime's own Event instead of event-listener.

### 📝 Documentation
- Show the Shuttle logo in the README and API docs.
- Add the Shuttle logo.
- Fix the README links to `Unblock` and to the license.
- Lead the README with the API rather than how it works.
- Say who calls `block_on` in AGENTS.md.
- Simplify the docs of the locks.
- Simplify the docs of the sockets.
- Simplify the docs of child processes.
- Simplify the docs of `unblock`, `Unblock` and `fs`.
- Simplify the docs of the MPMC channel.
- Simplify the docs of `Event` and the broadcast channel.
- Simplify the docs of the runtime API.
- Rewrite the README in plain, short sentences.
- Point at futures for combinators and I/O extension traits. #25
- Document async child processes.
- Document running work on several threads. #26
- Add CodSpeed as a sponsor in the README.
- Document the local and shared runtimes and the helper feature.
- README: Move runtime fetching out of the closure.
- README: Minor improvements.

### 🐛 Fixed
- Wait on select(2) on Apple's platforms, which can watch a terminal.
- Have a wait for readiness fail where the source cannot be watched.
- Turn away a descriptor the runtime watches already.
- Bound every wait to what each platform's wait takes.
- Drop a task's future before its handle hears of it, polled or not.
- Wake every task a notification reaches, whichever waker panics.
- Report a connect refused in between its two checks as refused.
- Keep a waiting runtime running under a nested block_on.
- Keep io/register-and-drop within the open-descriptor limit.

### ⚡️ Performance
- Watch for a child's exit through a pidfd on Linux.
- Wait on kqueue on the BSDs.
- Wait on epoll on Linux and Android.
- Run unblock's work on a pool of reused threads.
- Keep a notification's look clear of the lock's path.
- Fence a notification only where it would skip the lock.
- Spare the locks and the broadcast channel the event's fences.
- Let a notification that reaches nobody skip the event's lock. #8
- Cut the cost of spawning and running a task.

### ✅ Testing
- Test that a flagged, unwritten wake-up fails no wait.
- Give the pool of the double-panic test the long idle timeout.
- Benchmark a round trip among idle sockets.
- Give each wake benchmark of Event an event of its own.
- Start the rounds of a benchmark's threads together.
- Pin the threads of the benchmarks to CPUs of their own.
- Benchmark handing work to unblock.
- Benchmark the MPMC channel.
- Keep the broadcast test of an open channel from racing its receiver.
- Build the benchmarks with a single codegen unit.
- Time taking a listener apart from waking it.
- Benchmark an uncontended lock::Mutex.
- Race notifications against listeners being taken and dropped.
- Test and benchmark through the lock module.
- Benchmark the broadcast channel.
- Benchmark Event's listen and notify paths.
- Benchmark a connection's traffic on the runtime.
- Benchmark what the runtime costs.
