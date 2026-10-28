//! What a spawn hands back, and the tasks the runtime holds on to itself.

use std::{
    future::Future,
    io,
    pin::Pin,
    sync::Weak,
    task::{Context, Poll},
};

use zbus::runtime::traits;

use super::{Inner, lock};

/// A spawned task: dropping the handle cancels it.
pub struct Task<T> {
    task: async_task::Task<T>,
    // Only to reach the runtime when the task is detached, so it must not keep the runtime alive.
    inner: Weak<Inner>,
}

impl<T> Task<T> {
    /// A handle on `task`, which was spawned on the runtime `inner` belongs to.
    pub(super) fn new(task: async_task::Task<T>, inner: Weak<Inner>) -> Self {
        Self { task, inner }
    }
}

impl<T> Future for Task<T> {
    type Output = io::Result<T>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.task).poll(cx).map(Ok)
    }
}

impl<T> traits::TaskHandle<T> for Task<T>
where
    T: Send + 'static,
{
    /// Hands the task to the runtime, which holds it until it ends or the runtime is released.
    ///
    /// `async_task::Task::detach` is not what a detached task wants here: it would leave the task
    /// owned by nothing, and one that is parked on something that never fires would then be
    /// unreachable for good, along with the connection state its future holds. A runtime owns the
    /// tasks spawned on it and ends them when it ends, and this one is no different.
    fn detach(self) {
        let Some(inner) = self.inner.upgrade() else {
            // The runtime is gone, and dropping the task is all that cancelling it takes.
            return;
        };

        lock(&inner.detached).push(Box::new(self.task));
    }
}

/// A detached task the runtime holds on to, whatever that task produces.
///
/// Tasks of every output type wait in one list, and the only thing the runtime asks of one is
/// whether it has ended, so that is all this trait exposes.
pub(super) trait Detached: Send {
    fn is_finished(&self) -> bool;
}

impl<T> Detached for async_task::Task<T>
where
    T: Send,
{
    fn is_finished(&self) -> bool {
        async_task::Task::is_finished(self)
    }
}
