//! What a runtime is made of, and the loop that drives it on a thread inside `block_on`.
//!
//! A runtime is a scheduler, a reactor, and the half of it that other threads reach: the
//! [`Remote`], through which a task is queued to be polled and the thread driving the runtime is
//! woken inside its wait. The first two hold state of the runtime's own flavour, behind `Rc` and
//! `RefCell` or `Arc` and `Mutex`; the remote is reached from wakers, which may be woken on any
//! thread, and so is always of the second kind whatever the runtime's flavour.
//!
//! A runtime has no thread of its own. The thread inside [`Core::block_on`] drives it for as long
//! as it is inside the call: between two polls of its future it runs a batch of ready tasks and
//! then waits on the reactor, so what the runtime has to do is done on that thread. One thread at
//! a time drives a runtime, and a thread drives one runtime at a time; a thread-local marker of
//! which runtime a thread drives is what the two rules are checked against, and what spares a
//! wake on that very thread the write that would break a wait it is not in. Beside the marker,
//! each flavour keeps a record of the runtime itself, for what is spawned on the thread without a
//! handle to spawn it on (see [`set_driving`]).
//!
//! A shared runtime with a seat, which the `helper` feature's registries hand out, is driven
//! through that seat instead (see the `driver` module), and writes the same marker and record as
//! it does.

use std::{
    cell::Cell,
    collections::VecDeque,
    future::Future,
    io, mem,
    pin::pin,
    ptr::NonNull,
    sync::{
        Arc, Mutex, MutexGuard, PoisonError,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll, Wake, Waker},
    thread,
    time::Duration,
};

use crate::{
    Mode, Shared, coop, log::error, mode::sealed::Lock, poll::Poller, reactor::Reactor,
    scheduler::Scheduler,
};

/// What a runtime is made of, shared by every handle on it and by the thread that drives it.
///
/// Public in name only, in a module nobody outside can reach, because the sealed trait behind
/// [`Mode`] names it.
pub struct Core<M>
where
    M: Mode,
{
    pub(crate) scheduler: Scheduler<M>,
    pub(crate) reactor: Reactor<M>,
    /// The half of the runtime that wakers reach, from any thread.
    pub(crate) remote: Arc<Remote>,
    /// Whether a thread is inside `block_on` on this runtime, so that a second thread asking to
    /// drive it at the same time is told it cannot.
    driven: M::Lock<bool>,
    /// Who runs this runtime where no thread is inside `block_on` on it.
    #[cfg(feature = "helper")]
    pub(crate) seat: M::Seat,
}

impl<M> Core<M>
where
    M: Mode,
{
    /// A runtime with nothing to do.
    ///
    /// What can fail is the channel a wait is broken through, which this opens.
    pub(crate) fn new() -> io::Result<M::Ptr<Self>> {
        Ok(M::new_ptr(Self::unshared()?))
    }

    /// Runs `future` to completion on the calling thread, driving this runtime alongside it.
    ///
    /// Panics when the calling thread already drives a runtime, which is a call from inside a
    /// task that runtime is running, or from inside the future of another `block_on`: such a
    /// call could only wait for the thread it is on. Panics, too, when another thread is driving
    /// this runtime right now.
    ///
    /// Not a method, because the thread's record of what it drives is made from the pointer the
    /// runtime is shared through, which a method has no way to get hold of.
    pub(crate) fn block_on<F>(this: &M::Ptr<Self>, future: F) -> F::Output
    where
        F: Future,
    {
        // The marks outlive the future, which is dropped as this returns: whatever that future
        // held — a task, a registration, a timer — is gone while this thread still drives the
        // runtime, so its drop asks nobody for a wake-up this thread would only take out again.
        let _driving = Driving::<M>::enter(this);
        let mut future = pin!(future);
        let signal = Arc::new(Signal {
            woken: AtomicBool::new(false),
            remote: this.remote.clone(),
        });
        let waker = Waker::from(signal.clone());
        let mut cx = Context::from_waker(&waker);
        let mut failed_waits = 0u32;
        loop {
            // Lowered before the poll, so that a wake during it is seen by the round that
            // follows, and with a swap, so that the poll sees what the wakes before it were for.
            signal.woken.swap(false, Ordering::AcqRel);
            // Budgeted like a task, because the tasks run only between its polls: with a lot of
            // ready I/O in it, it would hold them up for as long as a task would hold up the rest.
            if let Poll::Ready(output) = coop::budgeted(|| future.as_mut().poll(&mut cx)) {
                return output;
            }
            this.round(&signal.woken, &mut failed_waits);
        }
    }

    /// One round for `block_on`: a batch, then one wait on the reactor, which waits for nothing
    /// where the future this thread is polling has been woken in the meantime, so that the poll
    /// of it comes next and what a source has for the tasks is reported all the same.
    pub(crate) fn round(&self, woken: &AtomicBool, failed_waits: &mut u32) {
        self.run_batch();
        // Read before the wait, so a wake that already landed is passed on as `at_once`.
        let woken = woken.load(Ordering::Acquire);

        self.wait(woken, failed_waits);
    }

    /// Polls up to `BATCH` ready tasks.
    pub(crate) fn run_batch(&self) {
        self.scheduler.run_batch(&mut [0; BATCH]);
    }

    /// One wait on the reactor: bounded by no time at all where a task is ready or `at_once`
    /// says the caller has something of its own to get back to, so that the round after it polls
    /// what is ready, and by nothing where neither holds, so that the thread sleeps until a
    /// source, a deadline or a notification has something for it.
    ///
    /// `failed_waits` counts the failures in a row, for the pause that keeps a wait which fails
    /// every time from becoming a spin; every waiter retries its own operation and sees its own
    /// error.
    pub(crate) fn wait(&self, at_once: bool, failed_waits: &mut u32) {
        let at_most = (at_once || self.remote.has_ready()).then_some(Duration::ZERO);
        match self.reactor.wait(at_most) {
            Ok(()) => *failed_waits = 0,
            Err(e) => {
                *failed_waits += 1;
                if *failed_waits == 1 {
                    error!("The runtime's wait failed: {}", e);
                }
                self.reactor.wake_everything();
                thread::sleep(Duration::from_millis(1 << (*failed_waits).min(10)));
            }
        }
    }

    /// A runtime with nothing to do, not yet shared: one of those only the threads inside
    /// `block_on` on it run.
    fn unshared() -> io::Result<Self> {
        let remote = Arc::new(Remote::new()?);

        Ok(Self {
            scheduler: Scheduler::new(remote.clone()),
            reactor: Reactor::new(remote.clone()),
            remote,
            driven: Lock::new(false),
            #[cfg(feature = "helper")]
            seat: Default::default(),
        })
    }
}

impl Core<Shared> {
    /// A shared runtime with nothing to do and a seat for whoever runs it, in no registry: one a
    /// helper thread runs where no thread is inside `block_on` on it.
    #[cfg(feature = "helper")]
    pub(crate) fn with_seat() -> io::Result<Arc<Self>> {
        let mut core = Self::unshared()?;
        core.seat = Some(Mutex::new(crate::driver::Seat::new()));

        Ok(Arc::new(core))
    }

    /// Sees to it that the work just handed over is run: for a runtime with a seat, a helper is
    /// started unless a thread is in the seat or about to take it. Called after that work is in
    /// place, never before.
    ///
    /// A runtime with no seat is run by the threads inside `block_on` on it alone, and the work
    /// waits for the next such call.
    pub(crate) fn ensure_progress(self: &Arc<Self>) {
        #[cfg(feature = "helper")]
        crate::driver::ensure_helper(self);
    }
}

/// How many ready tasks run between two looks at the reactor, so a busy task queue cannot
/// starve a socket or a timer.
const BATCH: usize = 32;

/// The half of a runtime that wakers reach: the queue of tasks ready to be polled, and the
/// channel that breaks the wait of the thread driving the runtime.
///
/// A waker may be woken on any thread, and may outlive the runtime it belongs to, so this is
/// shared through an [`Arc`] and guarded by a [`Mutex`] and atomics whatever the runtime's flavour.
/// A task is queued by its id rather than by a pointer to it: the task itself, future and all,
/// stays with the runtime, which may be one that never leaves its thread.
///
/// The poller lives here, both halves of its channel together, rather than with the scheduler and
/// the reactor: a wake may come after the runtime is gone, and a channel whose read half went with
/// the runtime would turn the write that wake makes into a broken pipe. A waker that outlives the
/// runtime keeps the channel open for as long as it lives, and so does a task's handle, which
/// holds the state its task's waker is made from.
pub(crate) struct Remote {
    /// The ids of the tasks ready to be polled, in the order they became ready.
    ///
    /// An id may be here for a task that has gone since — finished, or cancelled while it
    /// waited — and is skipped when its turn comes.
    ready: Mutex<VecDeque<u64>>,
    pub(crate) poller: Poller,
    /// Whether a wake-up is on its way to the thread driving the runtime, so that a `notify`
    /// which finds it up leaves its own write out: one wake-up in the channel ends a wait as
    /// surely as a hundred do.
    ///
    /// Raised by the `notify` that writes, before it writes, and lowered by the reactor's wait
    /// once the poll returns, whether that poll took the wake-up out or ended some other way and
    /// left it to end the next wait at once. Either way, whatever a `notify` turned away in the
    /// meantime had to say is looked at by the driving thread after the flag is down. The raising
    /// swap acquires the lowering store, so that a `notify` which finds the flag down writes after
    /// the wait that brought it down.
    pub(crate) wake_pending: AtomicBool,
}

impl Remote {
    /// Queues the task `id` to be polled, and wakes the thread driving the runtime to poll it.
    pub(crate) fn schedule(&self, id: u64) {
        lock(&self.ready).push_back(id);
        // Clear of the lock: the driving thread may take the id up the moment it hears of it.
        self.notify();
    }

    /// Takes up to `ids.len()` of the ids queued, oldest first, into `ids`, and says how many.
    pub(crate) fn take_ready(&self, ids: &mut [u64]) -> usize {
        let mut ready = lock(&self.ready);
        let taken = ready.len().min(ids.len());
        for (slot, id) in ids.iter_mut().zip(ready.drain(..taken)) {
            *slot = id;
        }

        taken
    }

    /// Puts `ids`, taken off the queue and not polled, back at its head, in the order given.
    pub(crate) fn requeue(&self, ids: &[u64]) {
        let mut ready = lock(&self.ready);
        for &id in ids.iter().rev() {
            ready.push_front(id);
        }
    }

    /// The id of the next task to poll, if any is ready.
    #[cfg(test)]
    pub(crate) fn next_ready(&self) -> Option<u64> {
        lock(&self.ready).pop_front()
    }

    /// Whether a task is waiting to be polled.
    pub(crate) fn has_ready(&self) -> bool {
        !lock(&self.ready).is_empty()
    }

    /// How many ids are queued, stale ones included.
    #[cfg(test)]
    pub(crate) fn ready_len(&self) -> usize {
        lock(&self.ready).len()
    }

    /// Wakes the thread driving this runtime inside its wait, unless called from that thread,
    /// which sees every change before its next wait anyway.
    ///
    /// One wake-up in the channel ends a wait as surely as a hundred do, so the rest are left
    /// unwritten: a burst of spawns, or a task cancelled alongside them, costs one write between
    /// two waits rather than one apiece.
    pub(crate) fn notify(&self) {
        if drives(self) {
            return;
        }
        // A previous `true` means a wake-up is on its way already, so this call is turned away
        // without writing to it again.
        if self.wake_pending.swap(true, Ordering::AcqRel) {
            return;
        }

        if self.poller.notify().is_err() {
            // The flag stands for a wake-up in the channel, so one that never got there takes it
            // down again and the next caller tries afresh. A channel that cannot be written to
            // leaves the driving thread waiting until its timeout, and there is nobody here to
            // tell about it who could do any better.
            self.wake_pending.store(false, Ordering::Release);
        }
    }

    /// Whether a wake-up this runtime wrote is in the channel, waiting to be taken out.
    #[cfg(test)]
    pub(crate) fn wake_pending(&self) -> bool {
        self.wake_pending.load(Ordering::Acquire)
    }

    /// An empty queue, with the channel a `notify` writes to open.
    fn new() -> io::Result<Self> {
        Ok(Self {
            ready: Mutex::new(VecDeque::new()),
            poller: Poller::new()?,
            wake_pending: AtomicBool::new(false),
        })
    }
}

/// A thread's time driving a runtime: marked here, unmarked when this is dropped, whether
/// `block_on` returns or unwinds.
struct Driving<'a, M>
where
    M: Mode,
{
    core: &'a M::Ptr<Core<M>>,
}

impl<'a, M> Driving<'a, M>
where
    M: Mode,
{
    /// Marks the calling thread as the one driving `core`.
    ///
    /// Panics where the thread drives a runtime already: the call comes from inside a task that
    /// runtime is running on this thread, or from inside the future of another `block_on`, and
    /// could only ever wait for itself. Panics, too, where another thread drives `core`: one
    /// thread at a time polls a runtime's tasks and waits on its reactor.
    fn enter(core: &'a M::Ptr<Core<M>>) -> Self {
        assert!(
            !drives_a_runtime(),
            "block_on called from a task this runtime is running: the call would wait for the \
             very thread it is on"
        );
        // Inside a call of the helper layer's `block_on`, whether or not that call holds its seat
        // at this moment: one that does not may be handed it at any turn, and one that does is
        // driving already. A call that blocked the thread here would keep that one from its
        // runtime either way, so it is turned away either way rather than by the timing of it.
        #[cfg(feature = "helper")]
        assert!(
            !crate::driver::in_block_on(),
            "block_on called from inside the future of another block_on: the call would keep the \
             thread from the runtime that one drives"
        );
        // Claimed under the lock and asserted once it is released, so that the panic leaves the
        // flag as it found it.
        let claimed = !mem::replace(&mut *core.driven.lock(), true);
        assert!(
            claimed,
            "block_on called on a runtime another thread is driving: a runtime is driven by one \
             thread at a time"
        );
        set_driving::<M>(Some(core));

        Self { core }
    }
}

impl<M> Drop for Driving<'_, M>
where
    M: Mode,
{
    fn drop(&mut self) {
        set_driving::<M>(None);
        *self.core.driven.lock() = false;
    }
}

/// The waker of the future a `block_on` polls.
struct Signal {
    /// Whether the future has been woken since it was last polled.
    ///
    /// Raised by each wake and lowered before each poll, both with `AcqRel` swaps rather than
    /// stores. The lowering acquires every wake since the one before it, so that the poll sees
    /// what they were for: a store reads nothing, and a store that raised the flag would cut the
    /// wakes before it off from the lowering. A wake that comes after a lowering acquires it in
    /// turn, and with it the end of the reactor wait before it, so that the `notify` the wake
    /// makes cannot take the wake-up that wait took out for one still to come.
    woken: AtomicBool,
    /// The runtime the future's thread drives, whose wait a wake from another thread breaks.
    remote: Arc<Remote>,
}

impl Wake for Signal {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    /// Marks the future woken and breaks the wait its thread may be in. A wake from the thread
    /// itself writes no notification, because that thread looks at the flag before its next
    /// wait.
    fn wake_by_ref(self: &Arc<Self>) {
        self.woken.swap(true, Ordering::AcqRel);
        self.remote.notify();
    }
}

/// Marks the calling thread as the one driving the runtime `core`, or, with `None`, as driving
/// none.
///
/// Written by whatever puts a thread in charge of a runtime and takes it out again: `block_on` on
/// a runtime with no seat, and the seat of one with a seat. The one record of which runtime a
/// thread drives, whichever of the two made it.
///
/// It is made twice over. The marker below names the runtime by the address of its remote, which
/// is all a wake needs to know whether it comes from the thread driving, and is read on the path
/// of every notification, so it is a plain cell. The flavour's own record,
/// [`Sealed::set_driven`](crate::mode::sealed::Sealed::set_driven), holds the runtime itself,
/// weakly, for what is spawned on this thread without a handle to spawn it on. Neither has a
/// destructor, so that both are written on a thread whose locals are being destroyed as on any
/// other, and the thread is on record for exactly as long as it is marked.
pub(crate) fn set_driving<M>(core: Option<&M::Ptr<Core<M>>>)
where
    M: Mode,
{
    DRIVING.with(|driving| driving.set(core.map(|core| NonNull::from(&*core.remote))));
    M::set_driven(core);
}

/// Whether the calling thread drives the runtime `remote` belongs to.
pub(crate) fn drives(remote: &Remote) -> bool {
    DRIVING.with(Cell::get) == Some(NonNull::from(remote))
}

/// Whether the calling thread drives a runtime, so that a `block_on` there, which could only wait
/// for the very thread it is on, is turned away.
pub(crate) fn drives_a_runtime() -> bool {
    DRIVING.with(Cell::get).is_some()
}

thread_local! {
    /// The remote of the runtime this thread drives, and nothing on a thread that drives none.
    ///
    /// The runtime is named by the address of its remote, which stands for it alone while a
    /// thread drives it: that thread holds the runtime, so the remote cannot be dropped and its
    /// place taken by another until the thread has stopped.
    static DRIVING: Cell<Option<NonNull<Remote>>> = const { Cell::new(None) };
}

/// The value behind a lock, taken whether or not a panic poisoned it.
pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
