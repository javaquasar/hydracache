//! The budget of I/O operations that one poll may complete before the next one yields.
//!
//! The scheduler is fair between tasks only as long as each task's poll returns. An operation
//! that is always ready, such as a read from a socket that already holds the bytes, never makes a
//! future return `Pending` on its own, so without a limit a loop over it runs to its end within
//! one poll, and every other task and every timer and source of the runtime waits for that. A
//! single task can starve the rest this way, though the ready queue is first-in first-out.
//!
//! So a runtime gives each poll it makes of a task, and of the future passed to `block_on`, a
//! budget of `BUDGET` operations. Every I/O operation that completes spends one. Once the budget
//! is gone, every operation that asks to run wakes its own task and returns `Pending` instead,
//! however many futures of the task ask and however often, which puts the task at the back of the
//! ready queue, behind the tasks that were there already. The poll that follows starts with a full
//! budget again.
//!
//! The budget is kept per thread, not per task: a poll runs on one thread from its start to its
//! end, and the operations it completes find the budget without being handed it. A thread that is
//! not inside a poll of a runtime of this crate has none. Whatever polls a future there, another
//! executor or plain code, is not ours to interrupt, and has no one to run the other tasks in
//! between, so an operation that finds no budget always proceeds and spends nothing.
//!
//! Only a runtime gives a poll a budget, but it is not the only thing that spends one: the
//! operations of the `fs` module do too, and that module is built without a runtime. So the
//! functions that check the budget and spend it are built for either, and the ones that set it for
//! the runtime alone. Where there is no runtime, no thread has a budget, and every operation
//! proceeds.
//!
//! A thread that is inside a poll has the budget of that poll, whoever polls the future in it. An
//! executor of another crate that a task runs, such as `futures::executor::block_on`, asks for no
//! budget of its own, and shares what is left of the task's. Once that is gone, an operation turns
//! such an executor away as it would the task, and one that blocks until its future is done polls
//! the future again as soon as it is woken, which is at once, inside the same poll of the task, and
//! is turned away again, for good. Such an executor has to run its future inside `unconstrained`,
//! which lifts the budget for the length of each poll of that future.
//!
//! Nothing here can tell that executor apart from a combinator, such as `select_all`, that polls
//! many futures of a task within one poll of it: both ask again and again inside the same poll,
//! each time with a waker that the refusal before woke, and a waker does not say which future it
//! was handed to. Anything that let the executor's operations through after a while would let the
//! combinator's through too, and with them as many operations as the task has futures.

use std::{
    cell::Cell,
    task::{Context, Poll},
};
#[cfg(feature = "runtime")]
use std::{future::poll_fn, pin::pin};

/// Runs `future` free of the budget of I/O operations that a runtime gives each poll of a task.
///
/// A runtime of this crate gives each poll of a task, and of the future passed to `block_on`, a
/// budget of I/O operations, as [the crate documentation](crate#tasks) describes. Once it is
/// spent, the next operation, and every one after it in that poll, yields: it wakes its task and
/// returns `Pending` without running, so that the other tasks have their turn. Each poll of
/// `future` that this makes lifts the budget: every operation in it runs, and none of them counts.
///
/// This is for a future that is polled in a way that never lets the task yield. A blocking
/// executor that a task runs, such as `futures::executor::block_on`, shares the budget of the
/// task's poll, and polls its future again as soon as it is woken, inside that same poll. Once the
/// budget is spent, every poll it makes yields and wakes it at once, and it never gets past the
/// operation. Running its future inside this lets the future do its I/O to the end. It is also for
/// code that must run its I/O through without yielding on the budget.
///
/// The budget of the poll this is awaited in is left as it was: the operations of `future` do not
/// spend it, and it holds again for what comes after each poll of `future`. This does not keep the
/// task from yielding otherwise: `future` still returns `Pending` where it waits, for a source
/// that is not ready, say. While `future` is polled, though, nothing else runs on the thread, so a
/// `future` with a lot of ready I/O holds up every other task, and the runtime's timers and I/O,
/// until it returns. Awaiting the future in the task, or spawning it, lets the other tasks run
/// meanwhile.
///
/// Outside a poll by a runtime of this crate there is no budget, and this changes nothing.
///
/// tokio has the same, as `tokio::task::unconstrained`.
///
/// # Example
///
/// A task has `futures`'s `block_on` read, a byte at a time, more bytes than the budget of a poll
/// lets it. Without `unconstrained`, the first read past the budget would yield to that executor,
/// which polls again at once, and the task would never get past that read.
///
/// ```
/// # #[cfg(unix)]
/// # fn main() -> std::io::Result<()> {
/// use std::io::{self, Read, Write};
///
/// use zruntime::{AsyncIo, LocalRuntime};
///
/// let runtime = LocalRuntime::new()?;
/// let (reader, mut writer) = io::pipe()?;
/// // In the pipe before the task starts, so that no read waits.
/// writer.write_all(&[7; 200])?;
/// let reader = AsyncIo::new(&runtime, reader)?;
///
/// let task = runtime.spawn("a reader", async move {
///     let reading = async {
///         let mut bytes = Vec::new();
///         for _ in 0..200 {
///             let mut byte = [0];
///             reader.read_with(|mut pipe| pipe.read(&mut byte)).await?;
///             bytes.push(byte[0]);
///         }
///
///         io::Result::Ok(bytes)
///     };
///
///     futures::executor::block_on(zruntime::unconstrained(reading))
/// });
///
/// let bytes = runtime.block_on(task).expect("the task does not panic")?;
/// assert_eq!(bytes, [7; 200]);
/// # Ok(())
/// # }
/// # #[cfg(not(unix))]
/// # fn main() {}
/// ```
#[cfg(feature = "runtime")]
pub async fn unconstrained<F>(future: F) -> F::Output
where
    F: Future,
{
    let mut future = pin!(future);

    poll_fn(|cx| {
        // Lifted for this poll of `future` alone, and put back as it returns or unwinds, so that
        // the poll this is awaited in goes on with what it had left.
        let _restore = Restore(LEFT.replace(None));

        future.as_mut().poll(cx)
    })
    .await
}

/// Runs `poll`, which polls a future on behalf of a runtime, with a full budget, and puts back the
/// budget the thread had when it returns.
///
/// The old budget comes back even when `poll` panics, so that a panic caught further up does not
/// leave this one behind. It is put back, rather than cleared, so that a poll made from inside
/// another that is budgeted too, such as the free `block_on` called from inside the future of
/// another call of it that holds no seat, ends with the budget of the poll it was made from.
#[cfg(feature = "runtime")]
pub(crate) fn budgeted<R>(poll: impl FnOnce() -> R) -> R {
    let _restore = Restore(LEFT.replace(Some(BUDGET)));

    poll()
}

/// Whether an I/O operation may run now: `Ready` where the poll has budget left or has none at all,
/// and `Pending`, after waking the task to poll it again, where the budget is spent.
///
/// An operation calls this before it runs, and [`spend`] once it has completed. A spent budget
/// stays spent for the rest of the poll, so every operation that asks after that is turned away,
/// however often it asks: one that has to run regardless runs inside `unconstrained`.
pub(crate) fn poll_proceed(cx: &mut Context<'_>) -> Poll<()> {
    if LEFT.get() != Some(0) {
        return Poll::Ready(());
    }
    // Woken rather than left to wait for something: nothing else will wake the task, which is
    // ready to go on as soon as the others have had their turn.
    cx.waker().wake_by_ref();

    Poll::Pending
}

/// Takes one operation out of the budget of the poll the thread is inside of, if it has one.
///
/// Called once an operation has completed, whether it succeeded or failed. An operation that
/// found the source not ready, and so completed nothing, spends nothing.
pub(crate) fn spend() {
    let Some(left) = LEFT.get() else {
        return;
    };
    // Saturating rather than checked: a budget that is spent stays spent, however an operation came
    // to complete without having asked [`poll_proceed`] first.
    LEFT.set(Some(left.saturating_sub(1)));
}

/// How many I/O operations one poll may complete before the next one yields.
///
/// Enough that the check costs next to nothing beside the operations it counts, and few enough that
/// a task with a lot of ready I/O gives up the thread long before the tasks behind it notice.
///
/// Named as a number in the documentation of `Registration::poll_io` and of the `fs` module, in the
/// Tasks and Files sections of the README and in `AGENTS.md`, so that a change here is a change
/// there too.
#[cfg(feature = "runtime")]
pub(crate) const BUDGET: u8 = 128;

/// Puts back the budget a thread had before a poll, when it is dropped: at the end of the poll, or
/// as a panic unwinds out of it.
///
/// `budgeted` sets a full budget for a poll of a runtime's, and `unconstrained` sets none for a
/// poll of the future it runs, and each puts back what was there before through this.
#[cfg(feature = "runtime")]
struct Restore(Option<u8>);

#[cfg(feature = "runtime")]
impl Drop for Restore {
    fn drop(&mut self) {
        LEFT.set(self.0);
    }
}

thread_local! {
    /// How many more operations the poll this thread is inside of may complete before the next one
    /// yields, and `None` on a thread that is inside no poll of a runtime of this crate, or inside
    /// a poll of a future that `unconstrained` runs.
    ///
    /// A `Cell` of a `Copy` value in a thread-local with a constant initialiser has no destructor,
    /// so that it can always be reached: a thread whose locals are being destroyed can run a
    /// `block_on` from the destructor of one of them, and that is budgeted like any other.
    static LEFT: Cell<Option<u8>> = const { Cell::new(None) };
}
