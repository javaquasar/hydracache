//! The timers a connection waits on, and the deadlines the runtime walks to fire them.

use std::{
    collections::BTreeMap,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Waker},
    time::{Duration, Instant},
};

use super::{Inner, Runtime, lock};

/// A timer the runtime wakes once its deadline has passed.
pub struct Sleep {
    inner: Arc<Inner>,
    deadline: Instant,
    // Handed out on the first poll, which is when the timer joins the runtime's map.
    id: Option<u64>,
}

impl Sleep {
    /// A timer that is due once `duration` has passed.
    pub(super) fn new(inner: Arc<Inner>, duration: Duration) -> Self {
        Self {
            inner,
            // This runtime's timers run on the standard clock, so a length of time is a deadline
            // on it; a runtime with a clock of its own would measure `duration` on that instead.
            deadline: Instant::now() + duration,
            id: None,
        }
    }
}

impl Future for Sleep {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let this = self.get_mut();
        if Instant::now() >= this.deadline {
            return Poll::Ready(());
        }

        let mut timers = lock(&this.inner.timers);
        let id = match this.id {
            Some(id) => id,
            None => {
                let id = timers.next_id;
                timers.next_id += 1;
                this.id = Some(id);

                id
            }
        };
        timers
            .pending
            .insert((this.deadline, id), cx.waker().clone());

        Poll::Pending
    }
}

impl Drop for Sleep {
    fn drop(&mut self) {
        if let Some(id) = self.id {
            lock(&self.inner.timers)
                .pending
                .remove(&(self.deadline, id));
        }
    }
}

impl Runtime {
    /// Wakes and forgets every timer whose deadline has passed.
    pub(super) fn wake_due_timers(&self) {
        let due = {
            let mut timers = lock(&self.0.timers);
            // Ids are handed out from zero upwards, so no timer carries `u64::MAX` and the split
            // leaves behind exactly the deadlines that have passed.
            let later = timers.pending.split_off(&(Instant::now(), u64::MAX));

            std::mem::replace(&mut timers.pending, later)
        };

        for waker in due.into_values() {
            waker.wake();
        }
    }
}

/// The timers waiting for their deadline, keyed so that two of the same deadline stay apart.
#[derive(Default)]
pub(super) struct Timers {
    pub(super) pending: BTreeMap<(Instant, u64), Waker>,
    next_id: u64,
}
