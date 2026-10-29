//! A connection on a runtime that is nothing but one thread, and what that runtime is left with.
//!
//! `runtime` is a `traits::Runtime` over the `polling` crate: readiness, timers and tasks, all
//! driven by the thread that calls `Runtime::run`, and nothing in it starts a thread. The tests
//! are grouped by what they establish about a connection built on it: `lifecycle` runs a whole
//! one and reads the process's thread count around it, `connect` follows the runtime's own
//! connects through the poller and the timer, `process` follows the wait for a child process
//! through the timer, `timeout` follows a method timeout's timer through the runtime's map, and
//! `teardown` checks that a released runtime gives everything its connections took back.
//!
//! Each group but `connect` and `process` needs a feature or two of zbus and gates itself, and the
//! runtime is compiled only where at least one of them is: without a user it would be dead code.
#![cfg(all(
    unix,
    any(all(feature = "proxy", feature = "service"), feature = "p2p")
))]

mod runtime;

mod connect;
mod lifecycle;
mod process;
mod teardown;
mod timeout;
