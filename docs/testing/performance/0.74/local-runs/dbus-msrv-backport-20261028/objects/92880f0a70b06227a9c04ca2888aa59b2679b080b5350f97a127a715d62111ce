//! A barrier that tasks wait at until enough of them have arrived.
//!
//! [`Barrier`] makes a number of tasks wait for each other and releases them together. The
//! [module documentation](super) describes what all the primitives of this module promise.

use std::{
    fmt,
    sync::{self, PoisonError},
};

use crate::Event;

/// A barrier that makes tasks wait for each other, without blocking the thread.
///
/// A barrier is made for `n` tasks. Each task that calls [`wait`](Barrier::wait) waits until the
/// `n`-th task arrives. Then all `n` tasks are released together. This is one round, and the
/// barrier is ready for the next round at once, so a single barrier serves tasks that meet over and
/// over.
///
/// Each round has one leader: the task whose arrival completes the round. It is not held up, and
/// its [`BarrierWaitResult`] says so, so that one task can do work that only one task should do
/// after the meeting.
///
/// A barrier made for zero tasks behaves like one made for one task, as with
/// [`std::sync::Barrier`]: every `wait` completes at once, and the task is the leader of its own
/// round.
///
/// The barrier needs no runtime and works under any executor. It is `Send` and `Sync`, and so are
/// the futures that wait at it. A task that panics instead of arriving leaves the others waiting
/// for it, and nothing is poisoned. A task that gives up its wait is no longer counted: see
/// [giving up a wait](crate::lock#giving-up-a-wait).
///
/// # Example
///
/// Three threads wait for each other. Exactly one of them is the leader:
///
/// ```
/// use std::{sync::Arc, thread};
///
/// use futures::executor::block_on;
/// use zruntime::lock::Barrier;
///
/// let barrier = Arc::new(Barrier::new(3));
/// let threads: Vec<_> = (0..3)
///     .map(|_| {
///         let barrier = barrier.clone();
///         thread::spawn(move || block_on(barrier.wait()).is_leader())
///     })
///     .collect();
///
/// let leaders = threads
///     .into_iter()
///     .map(|thread| thread.join().expect("the other thread did not panic"))
///     .filter(|&leader| leader)
///     .count();
///
/// assert_eq!(leaders, 1);
/// ```
pub struct Barrier {
    /// How many tasks make a round.
    n: usize,
    state: sync::Mutex<BarrierState>,
    /// Where the tasks of a round wait, notified once the round is complete. It is listened to and
    /// notified without its fences, through `Event::listen_unfenced` and
    /// `Event::notify_unfenced`: every check of the state is made under its lock, and every change
    /// to it under that lock too, with the notification after, which the lock then orders.
    tripped: Event,
}

impl Barrier {
    /// Creates a barrier for `n` tasks, with none waiting.
    ///
    /// A barrier for `0` tasks behaves like one for `1`. This is a `const fn`, so a barrier can be
    /// a `static`.
    ///
    /// # Example
    ///
    /// ```
    /// use futures::executor::block_on;
    /// use zruntime::lock::Barrier;
    ///
    /// static GATE: Barrier = Barrier::new(1);
    ///
    /// assert!(block_on(GATE.wait()).is_leader());
    /// ```
    pub const fn new(n: usize) -> Self {
        Self {
            n,
            state: sync::Mutex::new(BarrierState {
                arrived: 0,
                round: 0,
            }),
            tripped: Event::new(),
        }
    }

    /// Waits for the other tasks of the round to arrive.
    ///
    /// The task arrives when the future is first polled, not when it is created. The future
    /// completes once `n` tasks have arrived, including this one. The `n`-th task to arrive
    /// completes at once. It is the leader of the round, and its [`BarrierWaitResult`] says so. The
    /// other tasks complete when the `n`-th task arrives. The barrier is then ready for the next
    /// round.
    ///
    /// # Cancel safety
    ///
    /// Dropping the future after its first poll and before it completes gives up the wait. The task
    /// is no longer counted as arrived, unless the round has completed by then. See
    /// [giving up a wait](crate::lock#giving-up-a-wait).
    ///
    /// # Example
    ///
    /// One thread drives two waits together. Each wait needs the other to have started:
    ///
    /// ```
    /// use futures::{executor::block_on, future::join};
    /// use zruntime::lock::Barrier;
    ///
    /// let barrier = Barrier::new(2);
    ///
    /// let (first, second) = block_on(join(barrier.wait(), barrier.wait()));
    ///
    /// // One of the two led the round.
    /// assert_ne!(first.is_leader(), second.is_leader());
    /// ```
    pub async fn wait(&self) -> BarrierWaitResult {
        let Some(arrival) = self.arrive() else {
            return BarrierWaitResult { leader: true };
        };
        loop {
            // Listen before re-checking so a trip between the check and the wait is seen.
            let listener = self.tripped.listen_unfenced();
            if arrival.round_is_over() {
                return BarrierWaitResult { leader: false };
            }
            listener.await;
        }
    }

    /// Counts a task in, handing back what makes the count good for as long as the task waits, or
    /// `None` where this arrival completes the round, which it then starts the next of.
    ///
    /// A round that is complete releases every task that arrived in it: they are notified once
    /// the state's lock is let go of, and find the round over on their next check.
    fn arrive(&self) -> Option<Arrival<'_>> {
        let mut state = lock(&self.state);
        // Cannot overflow: the count stays below `n` between arrivals, and `n` is a `usize`.
        state.arrived += 1;
        if state.arrived < self.n {
            return Some(Arrival {
                barrier: self,
                round: state.round,
            });
        }
        state.arrived = 0;
        // Cannot overflow: at one round a nanosecond, a `u64` lasts for centuries.
        state.round += 1;
        drop(state);

        self.tripped.notify_unfenced(usize::MAX);

        None
    }
}

impl fmt::Debug for Barrier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let waiting = lock(&self.state).arrived;

        f.debug_struct("Barrier")
            .field("n", &self.n)
            .field("waiting", &waiting)
            .finish()
    }
}

/// The result of waiting at a [`Barrier`]: whether the task led its round.
///
/// Created by [`Barrier::wait`].
pub struct BarrierWaitResult {
    leader: bool,
}

impl BarrierWaitResult {
    /// Whether this task is the leader of its round.
    ///
    /// The leader is the task whose arrival completed the round, which is the last task to arrive.
    /// Each round has exactly one leader.
    ///
    /// # Example
    ///
    /// ```
    /// use futures::executor::block_on;
    /// use zruntime::lock::Barrier;
    ///
    /// // A barrier for one task completes every round with the task that arrives.
    /// let barrier = Barrier::new(1);
    ///
    /// assert!(block_on(barrier.wait()).is_leader());
    /// ```
    pub fn is_leader(&self) -> bool {
        self.leader
    }
}

impl fmt::Debug for BarrierWaitResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BarrierWaitResult")
            .field("is_leader", &self.leader)
            .finish()
    }
}

/// What every `wait` decides on.
///
/// The blocking mutex around it is held for the few instructions that check and update it, never
/// across an await.
struct BarrierState {
    /// The tasks that have arrived in the current round and wait for it to be complete.
    arrived: usize,
    /// How many rounds have been completed, which is what names the current one.
    round: u64,
}

/// A task that waits at a [`Barrier`], counted in the round it arrived in for as long as its `wait`
/// future lives.
///
/// A round that is complete is over for everyone in it, and counts no one any more, so the count
/// of a task is taken back only where its round is still the current one: a `wait` dropped before
/// its round was completed leaves the barrier needing the arrival it had made, and one dropped
/// after leaves the next round's count alone.
struct Arrival<'a> {
    barrier: &'a Barrier,
    /// The round this task arrived in.
    round: u64,
}

impl Arrival<'_> {
    /// Whether the round this task arrived in has been completed.
    fn round_is_over(&self) -> bool {
        lock(&self.barrier.state).round != self.round
    }
}

impl Drop for Arrival<'_> {
    fn drop(&mut self) {
        let mut state = lock(&self.barrier.state);
        if state.round != self.round {
            return;
        }
        // Counted in this round, which has not been completed since, so the count includes this
        // task. Nobody waits for the count to fall, so there is no one to notify.
        state.arrived -= 1;
    }
}

/// The state behind the barrier, taken whether or not a panic poisoned it.
///
/// Poisoning says nothing here: no path can panic part of the way through its changes to the
/// state, so a panic elsewhere cannot leave it half-updated.
fn lock(state: &sync::Mutex<BarrierState>) -> sync::MutexGuard<'_, BarrierState> {
    state.lock().unwrap_or_else(PoisonError::into_inner)
}
