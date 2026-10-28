# zruntime

This build of zruntime has no runtime. It has [`Event`], a notification that tasks can wait for,
and works under any executor.

Call [`Event::listen`] to get an [`EventListener`], then await the listener. Code that changes what
a task waits for, for example by releasing a lock or filling a queue, calls [`Event::notify`]. That
wakes as many listeners as it asks for, oldest first. A listener is a plain future, and an event
can be notified from any thread.

## Cargo features

* `runtime` (default, off in this build): `Runtime`, `LocalRuntime`, `SharedRuntime`, tasks,
  timers, `AsyncIo` and `Registration`.
* `event` (default, on in this build): [`Event`] and [`EventListener`].
* `tracing` (default): log through the `tracing` crate. Without the runtime, there is nothing to
  log.
* `helper`: per-thread runtimes, and a helper thread that keeps their work running between
  `block_on` calls. Implies `runtime`.
* `lock`: the `lock` module, with an async `Mutex`, `RwLock`, `Semaphore`, `Barrier` and
  `OnceCell`. Implies `event`.
* `mpmc`: the `mpmc` module, a channel that gives each message to one receiver. Implies `event`.
* `broadcast`: the `broadcast` module, a channel that gives each message to every receiver.
  Implies `event`.
* `unblock`: `unblock`, which runs blocking work on a pool of threads, and `Unblock`, which makes a
  blocking I/O handle async that way.
* `fs`: the `fs` module, async filesystem access that runs each operation on the thread that
  polls it.
* `tcp`: `TcpListener` and `TcpStream` in the `net` module. Implies `runtime`.
* `udp`: `UdpSocket` in the `net` module. Implies `runtime`.
* `unix`: the `net::unix` module, on unix only. Implies `runtime`.
* `process`: the `process` module, async child processes. Implies `runtime` and `unblock`.

`lock`, `mpmc`, `broadcast`, `unblock` and `fs` need no runtime either. To build only what needs no
runtime, turn off the default features and enable only what you use, such as
`features = ["lock"]`. [The documentation on docs.rs](https://docs.rs/zruntime) is built with every
feature on, and covers the runtime as well.
