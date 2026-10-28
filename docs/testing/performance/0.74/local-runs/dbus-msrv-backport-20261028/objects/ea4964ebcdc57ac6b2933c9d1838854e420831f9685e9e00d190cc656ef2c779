//! Running a benchmark's routines inside `zruntime::block_on`, on the runtime of the thread.

use std::future::Future;

use criterion::async_executor::AsyncExecutor;
use zruntime::SharedRuntime;

/// Runs a routine's future to completion inside `zruntime::block_on`.
pub struct ZruntimeExecutor;

impl AsyncExecutor for ZruntimeExecutor {
    fn block_on<T>(&self, future: impl Future<Output = T>) -> T {
        zruntime::block_on(future)
    }
}

/// A handle on the runtime this thread's `block_on` calls drive, brought into being in a
/// `block_on` of its own and kept alive by the caller so that every later `block_on` on this
/// thread resolves to the same runtime rather than a fresh one.
pub fn runtime_handle() -> SharedRuntime {
    zruntime::block_on(async { SharedRuntime::current().expect("a runtime for this thread") })
}
