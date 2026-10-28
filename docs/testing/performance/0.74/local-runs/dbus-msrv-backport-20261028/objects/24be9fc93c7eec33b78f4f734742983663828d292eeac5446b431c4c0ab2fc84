//! The tasks of one runtime, and the polling of those that are ready.
//!
//! A scheduler holds every task spawned on it and hands them out a batch at a time to be polled,
//! on whichever thread calls [`Scheduler::run_batch`]. It has no thread of its own: the thread
//! driving the runtime calls that method for as long as there is anything to run.
//!
//! Every task lives in a slot of the scheduler's own, under an id that is never handed out
//! again, and holds its future there while it waits. Waking a task, from any thread, puts its id
//! on the runtime's ready queue and breaks the wait of the thread driving the runtime, which is
//! how a thread waiting with nothing to do learns that it has a task to poll. The waker holds the
//! id rather than the task's future: a waker has to be `Send` and `Sync`, and the future need be
//! neither. A wake for a task that is already queued does nothing, so a task woken three times is
//! polled once; the task is marked unqueued right before it is polled, so a wake that arrives
//! during the poll queues it again. An id whose task has gone by the time it comes up is skipped.
//!
//! The waker is made once, as the task is spawned, and waits in the slot beside the future, both
//! taken out for a poll and put back after it. On a shared runtime it points at what the task and
//! its handle share, which is `Send` and `Sync` there, rather than at an allocation of its own: a
//! spawn that allocates twice rather than three times is most of what spawning costs.
//!
//! [`spawn`] hands back a handle that resolves to what the task produced. The task's future is
//! wrapped before it is stored, and the wrapper is what hands the outcome over: the future's
//! output, or an error where it panicked, or — through a guard it holds, as it is dropped
//! unfinished — an error where the runtime went away with it or the task was cancelled. Whichever
//! it is, it is handed over once the future has been dropped. Dropping the handle cancels the
//! task: the future is dropped there and then if nobody is polling it, and by the poll under way
//! otherwise. A handle can also cancel the task and stay, to learn when the future is gone: the
//! future is dropped just as for a drop of the handle, but the outcome is left for the wrapper to
//! hand over, which tells the handle that the future is gone, and hands it the output where the
//! task had finished first. A task whose outcome is of no further interest is detached instead,
//! and runs until it ends.
//!
//! A task that panics takes nothing with it. The panic is caught where the task is polled, the
//! task ends there, its handle reports the failure, and the thread that polled it carries on
//! with the next task.
//!
//! No lock is held while a future is polled or dropped, nor while a waker is woken or dropped,
//! because all of these run code that may spawn a task, cancel one or wake another on this very
//! scheduler — the destructor of a future is where its last piece of work is handed off — and
//! that code takes the locks that would be held. On a local runtime the locks are `RefCell`s, and
//! the same rule is what keeps a borrow from ever finding another one out. The table of tasks and
//! the outcome a handle waits on each have a lock of their own, and neither is ever taken while
//! the other is held.

use std::{
    borrow::{Borrow, Cow},
    collections::HashMap,
    fmt,
    future::{Future, poll_fn},
    hash::{BuildHasherDefault, Hasher},
    io, mem,
    panic::{AssertUnwindSafe, Location, catch_unwind, resume_unwind},
    pin::{Pin, pin},
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll, Wake, Waker},
    thread,
};

use crate::{
    Local, Mode, Shared, coop,
    log::error,
    mode::sealed::Lock,
    runtime::{Core, Remote},
};

/// The tasks of one runtime: what has been spawned on it and has not ended yet.
pub(crate) struct Scheduler<M>
where
    M: Mode,
{
    tasks: M::Lock<Tasks<M>>,
    /// Where the ids of the tasks ready to be polled are queued.
    remote: Arc<Remote>,
}

impl<M> Scheduler<M>
where
    M: Mode,
{
    /// An empty scheduler, which queues its tasks on `remote`.
    pub(crate) fn new(remote: Arc<Remote>) -> Self {
        Self {
            tasks: Lock::new(Tasks {
                slots: HashMap::default(),
                next_id: 0,
            }),
            remote,
        }
    }

    /// Polls up to `ids.len()` ready tasks, whose ids are taken off the queue into `ids` under one
    /// lock rather than one apiece.
    ///
    /// An id whose task has gone since it was queued counts as one polled: it took its place in
    /// the queue, and the batch it was taken in is that much closer to its end.
    ///
    /// The ids taken are this call's alone until it has polled them: a task among them that is
    /// woken meanwhile is still marked queued, and the poll it comes up for sees what the wake was
    /// for, while one cancelled meanwhile is skipped as it comes up. Nobody misses them in the
    /// queue: the thread driving the runtime looks at it between batches, not during one, and
    /// whether a runtime has work left counts the tasks that are alive as well as the ids that
    /// are queued.
    pub(crate) fn run_batch(&self, ids: &mut [u64]) {
        let mut budget = ids.len();
        // Taken again once those in hand are polled, so that a task those polls woke runs in this
        // batch, as it would where ids were taken one at a time, rather than after a wait.
        while budget > 0 {
            let taken = self.remote.take_ready(&mut ids[..budget]);
            if taken == 0 {
                break;
            }
            budget -= taken;
            let mut left = Requeue {
                remote: &self.remote,
                ids: &ids[..taken],
            };
            while let Some((&id, rest)) = left.ids.split_first() {
                left.ids = rest;
                self.run(id);
            }
        }
    }

    /// Polls one ready task. `false` when the queue was empty.
    #[cfg(test)]
    pub(crate) fn run_one(&self) -> bool {
        let Some(id) = self.remote.next_ready() else {
            return false;
        };
        self.run(id);

        true
    }

    /// Polls the task `id`, taken off the ready queue, unless it is gone.
    fn run(&self, id: u64) {
        let taken = {
            let mut tasks = self.tasks.lock();
            tasks.slots.get_mut(&id).and_then(|slot| {
                match mem::replace(slot, Slot::Running { cancelled: false }) {
                    Slot::Idle { future, waker } => Some((future, waker)),
                    // Nothing to poll: the future is already out of its slot. Only a poll takes
                    // it out, and nothing polls from inside a poll, but an entry like that is
                    // put back rather than trusted to be impossible.
                    running => {
                        *slot = running;

                        None
                    }
                }
            })
        };
        // A task that is gone — finished, or cancelled while it waited — leaves its id behind in
        // the queue.
        let Some((mut future, waker)) = taken else {
            return;
        };

        // The queue entry this was taken off is spent by the wrapper's poll, which marks the task
        // unqueued before it polls the task's own future (see [`task`]). The wrapper catches the
        // panics of the task's own future, so what is caught here is what lies outside that: the
        // wake of whoever joins the task, say. The poll has a budget of I/O operations, so that a
        // task with a lot of ready I/O gives the others their turn (see the `coop` module).
        let polled = catch_unwind(AssertUnwindSafe(|| {
            coop::budgeted(|| Pin::new(&mut future).poll(&mut Context::from_waker(&waker)))
        }));
        let ended = match polled {
            Ok(Poll::Ready(())) => true,
            Ok(Poll::Pending) => false,
            Err(payload) => {
                error!("A task panicked outside its own future");
                // Nothing carries the panic any further, so its payload ends here. The payload's
                // destructor is the task's code as much as the future is, and is contained the
                // same way.
                dispose(payload);

                true
            }
        };

        // A future that is finished with is taken out of its slot here and dropped below, with
        // no lock held, and its waker with it.
        let finished = {
            let mut tasks = self.tasks.lock();
            let Some(slot) = tasks.slots.get_mut(&id) else {
                unreachable!("a task being polled keeps its slot");
            };
            let Slot::Running { cancelled } = *slot else {
                unreachable!("only this call takes a slot out of `Running`");
            };
            if ended || cancelled {
                tasks.slots.remove(&id);

                Some((future, waker))
            } else {
                *slot = Slot::Idle { future, waker };

                None
            }
        };
        if let Some((future, waker)) = finished {
            // A destructor is free to panic and free to spawn, so it runs here: caught, and clear
            // of every lock a spawn of its own would take. The waker may be the last hold on what
            // the task and its handle share, and so on the output of a detached task.
            dispose(future);
            dispose(waker);
        }
    }

    /// How many spawned futures have neither finished nor been cancelled.
    #[cfg(any(test, feature = "helper"))]
    pub(crate) fn live_tasks(&self) -> usize {
        self.tasks.lock().slots.len()
    }

    /// The waker of the task `id`, which is waiting to be polled.
    #[cfg(test)]
    pub(crate) fn waker(&self, id: u64) -> Waker {
        match &self.tasks.lock().slots[&id] {
            Slot::Idle { waker, .. } => waker.clone(),
            Slot::Running { .. } => unreachable!("the task is not being polled"),
        }
    }
}

impl<M> Drop for Scheduler<M>
where
    M: Mode,
{
    fn drop(&mut self) {
        // Nothing can reach the scheduler any more, so its table is emptied without a lock, and
        // each future is dropped with no borrow of it out. Dropping a future unfinished is what
        // fails its handle: the guard the task's wrapper holds sees to it as it goes.
        let slots: Vec<Slot<M>> = self
            .tasks
            .get_mut()
            .slots
            .drain()
            .map(|(_, slot)| slot)
            .collect();
        for slot in slots {
            dispose(slot);
        }
    }
}

/// Queues `future` on the local runtime `core`, as a task named `name`, and hands back the handle
/// that joins or cancels it.
pub(crate) fn spawn_local<F>(
    core: &Rc<Core<Local>>,
    name: Name,
    future: F,
) -> JoinHandle<F::Output, Local>
where
    F: Future + 'static,
    F::Output: 'static,
{
    spawn(core, move |task_waker| {
        let (join, wrapper) = task::<Local, _>(name, task_waker, future);
        let waker = Waker::from(join.waker.clone());
        let wrapper: Pin<Box<dyn Future<Output = ()>>> = Box::pin(wrapper);

        (join, waker, wrapper)
    })
}

/// Queues `future` on the shared runtime `core`, as a task named `name`, and hands back the
/// handle that joins or cancels it.
pub(crate) fn spawn_shared<F>(
    core: &Arc<Core<Shared>>,
    name: Name,
    future: F,
) -> JoinHandle<F::Output, Shared>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    spawn(core, move |task_waker| {
        let (join, wrapper) = task::<Shared, _>(name, task_waker, future);
        // What the task and its handle share is what its waker points at, so that one allocation
        // serves both.
        let waker = Waker::from(join.clone());
        let wrapper: Pin<Box<dyn Future<Output = ()> + Send>> = Box::pin(wrapper);

        (join, waker, wrapper)
    })
}

/// Queues the task `make` builds on `core`, and hands back the handle that joins or cancels it.
///
/// `make` is handed the state of the task's waker, id and all, and makes of it what the task and
/// its handle share, the task's waker, and its future, wrapped by [`task`] and boxed as the
/// runtime's flavour takes it.
fn spawn<M, T>(
    core: &M::Ptr<Core<M>>,
    make: impl FnOnce(TaskWaker) -> (M::Ptr<Join<M, T>>, Waker, M::BoxFuture),
) -> JoinHandle<T, M>
where
    M: Mode,
{
    let (id, join) = {
        let mut tasks = core.scheduler.tasks.lock();
        let id = tasks.next_id;
        tasks.next_id += 1;
        // Made under the lock, which the id is minted under, rather than after a second one:
        // nothing of the caller's runs here, only the move of its future into place.
        let (join, waker, future) = make(TaskWaker {
            id,
            // Queued directly below rather than through a wake: the task is reachable by nobody
            // else yet, so it is safe to know without asking that it is not already queued.
            queued: AtomicBool::new(true),
            remote: core.remote.clone(),
        });
        tasks.slots.insert(id, Slot::Idle { future, waker });

        (id, join)
    };
    core.remote.schedule(id);

    JoinHandle {
        id,
        core: M::downgrade(core),
        join,
        detached: false,
        cancelled: false,
    }
}

/// The ids a batch has taken off the ready queue and not polled yet, put back at its head should
/// the batch unwind: a poll contains every panic of the task it polls, but a task whose id went
/// missing would never be polled again, so none is left to that.
struct Requeue<'a> {
    remote: &'a Remote,
    ids: &'a [u64],
}

impl Drop for Requeue<'_> {
    fn drop(&mut self) {
        if !self.ids.is_empty() {
            self.remote.requeue(self.ids);
        }
    }
}

/// Wraps `future` as a task named `name`, whose waker is `task_waker`: the future to store, and
/// what its handle and it share.
///
/// The wrapper marks the task unqueued before each poll of `future`, polls it with its panics
/// caught, drops it once it is done with, and hands its outcome over to the handle: the output,
/// or an error where it panicked. It holds a guard that hands an error over instead, should the
/// wrapper itself be dropped unfinished.
///
/// The future is left unerased, so that the caller, which knows the concrete type and so whether
/// it is `Send`, boxes it as its runtime's flavour takes it.
fn task<M, F>(
    name: Name,
    task_waker: TaskWaker,
    future: F,
) -> (
    M::Ptr<Join<M, F::Output>>,
    impl Future<Output = ()> + 'static,
)
where
    M: Mode,
    F: Future + 'static,
    F::Output: 'static,
{
    let join = M::new_ptr(Join {
        name,
        waker: task_waker.into(),
        state: Lock::new(JoinState {
            output: None,
            done: false,
            join_waker: None,
        }),
    });
    let unpolled = Unpolled {
        future: Some(future),
        guard: CompleteOnDrop::<M, F::Output>(join.clone()),
    };
    let wrapper = async move {
        // The guard is dropped after the future whether the wrapper goes before its first poll or
        // after it, so that a task cancelled, or dropped with its runtime, drops its future before
        // the guard hears of it, as a task that ends does. Before the first poll, the wrapper holds
        // the two in the one value it captured, whose fields go in the order they are declared in.
        // After it, they are the wrapper's locals, which go in the reverse of the order they are
        // declared in, so the future is pinned in a local declared after the guard's. The value is
        // taken whole before it is taken apart, so that the wrapper captures it rather than each
        // of its fields on its own, in an order nothing promises.
        let unpolled = unpolled;
        let guard = unpolled.guard;
        let mut future = pin!(unpolled.future);
        let polled = catch_unwind_polls(guard.0.task_waker(), future.as_mut()).await;
        // Done with, and dropped here, before the handle hears of the outcome: whoever joins the
        // task finds whatever the future held already gone. The value is moved out of `polled`
        // ahead of the drop, so a future whose destructor panics still hands its value back.
        let dropped = catch_unwind(AssertUnwindSafe(|| future.set(None)));
        let output = match polled {
            Ok(value) => Ok(value),
            Err(payload) => {
                error!("The task {} panicked", guard.0.name);
                // Nothing carries the panic any further, so its payload ends here. That
                // payload's destructor is the task's code as much as the future is, and is
                // contained the same way.
                dispose(payload);

                Err(io::Error::other("the task panicked"))
            }
        };
        if let Err(payload) = dropped {
            dispose(payload);
        }

        guard.complete(output);
    };

    (join, wrapper)
}

/// A task's future and the guard that hands its outcome over, as the task's wrapper holds them
/// until its first poll: in one value, whose fields are dropped in the order they are declared
/// in, the future before the guard.
struct Unpolled<F, G> {
    future: Option<F>,
    guard: G,
}

/// Joins a spawned task, and cancels it when dropped unless [`JoinHandle::detach`] was called;
/// [`JoinHandle::cancel`] cancels it and waits for its future to be gone.
pub(crate) struct JoinHandle<T, M>
where
    M: Mode,
{
    id: u64,
    /// Weakly, so that a handle nobody polls keeps nothing of a runtime alive but the outcome it
    /// is waiting on.
    core: M::Weak<Core<M>>,
    join: M::Ptr<Join<M, T>>,
    detached: bool,
    /// Whether [`JoinHandle::cancel`] has cancelled the task already, which leaves the drop of
    /// the handle nothing to do but let go of the outcome.
    cancelled: bool,
}

impl<T, M> JoinHandle<T, M>
where
    M: Mode,
{
    /// Lets the task run to completion on its own.
    ///
    /// Marks the outcome settled, so that the task drops its output as it hands it over rather
    /// than leave it with what the task and its handle share: on a shared runtime, the task's
    /// waker holds that too, and a waker somebody kept would keep the output alive with it.
    pub(crate) fn detach(mut self) {
        self.detached = true;
        self.settle();
    }

    /// Cancels the task, as a drop of the handle does, and hands back a future that resolves once
    /// the task's future is gone: to the output, where the task finished before it was
    /// cancelled, and to `None` otherwise.
    ///
    /// Unlike a drop, this leaves the outcome for the task to settle: the guard its wrapper holds
    /// settles it once the future has been dropped, here or after the poll under way, and wakes
    /// whoever waits on it, which by then is the future handed back.
    pub(crate) fn cancel(mut self) -> impl Future<Output = Option<T>> {
        // Marked first, so that a destructor panicking out of the stop below drops the handle as
        // one whose task is cancelled already.
        self.cancelled = true;
        let unsettled = {
            let mut state = self.join.state.lock();
            // Finished, or failed as its runtime went: nothing is left to stop, and the outcome
            // is there for the taking.
            (!state.done).then(|| state.join_waker.take())
        };
        if let Some(join_waker) = unsettled {
            self.stop(join_waker);
        }

        poll_fn(move |cx| self.poll_cancelled(cx))
    }

    /// The task's outcome, once it has one; a wake of `cx`'s waker once it does, otherwise.
    pub(crate) fn poll_join(&self, cx: &mut Context<'_>) -> Poll<io::Result<T>> {
        // Cloned before the lock is taken: a waker's clone is somebody else's code, which is not
        // to run with a lock of this runtime's held.
        let waker = cx.waker().clone();
        let replaced = {
            let mut state = self.join.state.lock();
            if let Some(output) = state.output.take() {
                return Poll::Ready(output);
            }

            state.join_waker.replace(waker)
        };
        // Clear of the lock, for the same reason.
        drop(replaced);

        Poll::Pending
    }

    /// Whether the task has ended, told without polling it or taking its output.
    ///
    /// For a handle that is still here, neither detached nor dropped, the outcome is settled by
    /// the task alone: by its wrapper, once the future has completed or panicked and been dropped,
    /// and by the guard the wrapper holds, which goes after the future where the runtime goes
    /// with the task. Either way, a settled outcome means the future, and everything it held, is
    /// gone. It stays settled once the output has been taken.
    pub(crate) fn is_finished(&self) -> bool {
        self.join.state.lock().done
    }

    /// What the task cancelled by [`JoinHandle::cancel`] hands back once its future is gone: its
    /// output, where it finished before it was cancelled, and `None` otherwise; a wake of `cx`'s
    /// waker once the future is gone, until then.
    ///
    /// An outcome settled before the cancellation may have had its output taken already, by a
    /// poll of the handle that resolved: there is nothing left to hand back then but `None`, and
    /// nothing to wait for.
    fn poll_cancelled(&self, cx: &mut Context<'_>) -> Poll<Option<T>> {
        // Cloned before the lock is taken: a waker's clone is somebody else's code, which is not
        // to run with a lock of this runtime's held.
        let waker = cx.waker().clone();
        let replaced = {
            let mut state = self.join.state.lock();
            if state.done {
                let output = state.output.take();
                drop(state);
                // Clear of the lock: the error a task that did not finish leaves, which is
                // handed on to nobody, is dropped here.
                return Poll::Ready(output.and_then(Result::ok));
            }

            state.join_waker.replace(waker)
        };
        // Clear of the lock, for the same reason.
        drop(replaced);

        Poll::Pending
    }

    /// Stops the task: drops its future here where nobody is polling it, and has whoever is
    /// polling it drop it once that poll returns otherwise. Wakes `join_waker`, which an earlier
    /// poll of the handle registered, and tells the thread driving the runtime.
    ///
    /// A future whose destructor panics here hands the panic on to the caller, once the task is
    /// closed off either way.
    fn stop(&self, join_waker: Option<Waker>) {
        let Some(core) = M::upgrade(&self.core) else {
            // The runtime is on its way out, and its future with it.
            if let Some(waker) = join_waker {
                waker.wake();
            }

            return;
        };
        let removed = {
            let mut tasks = core.scheduler.tasks.lock();
            let running = match tasks.slots.get_mut(&self.id) {
                // Whoever is polling the task drops the future once that poll returns.
                Some(Slot::Running { cancelled }) => {
                    *cancelled = true;

                    true
                }
                Some(_) => false,
                // Gone already: it ended since its outcome was looked at, and the thread that
                // polled it last drops the future, or has dropped it.
                None => true,
            };

            (!running).then(|| tasks.slots.remove(&self.id)).flatten()
        };
        // On the thread that let the handle go or cancelled the task, with no lock held: a
        // destructor that panics here panics where that was done, and one that spawns is served
        // as any other caller is.
        let dropped = removed.map(|slot| catch_unwind(AssertUnwindSafe(move || drop(slot))));
        if let Some(waker) = join_waker {
            // Outside the lock: whoever registered it, polling this very handle earlier and
            // getting `Pending`, may be polled to completion right here.
            waker.wake();
        }
        // Even where nothing was queued: a thread waiting with this as its last task learns
        // from it that it has nothing left to wait for, and a helper thread that it can retire.
        core.remote.notify();
        // Carried on from here, so that the panic is still the caller's to see while the task it
        // belonged to is closed off either way: the notification above is what lets an idle
        // helper retire, and somebody's destructor panicking is no reason to leave one up.
        if let Some(Err(payload)) = dropped {
            resume_unwind(payload);
        }
    }

    /// Settles the outcome where the task has not settled it yet, and lets go of what it handed
    /// over: the task then finds nobody to tell, and drops its output as it hands it over rather
    /// than leave it with what the task and its handle share.
    fn settle(&self) {
        let (output, join_waker) = {
            let mut state = self.join.state.lock();
            state.done = true;

            (state.output.take(), state.join_waker.take())
        };
        // Clear of the lock: an output's destructor is the task's code, and a waker's somebody
        // else's.
        drop(output);
        drop(join_waker);
    }
}

impl<T, M> Drop for JoinHandle<T, M>
where
    M: Mode,
{
    fn drop(&mut self) {
        if self.detached {
            return;
        }
        if self.cancelled {
            // Cancelled already, by `cancel`, and nobody waits on the outcome any more: it is
            // settled here, should the task not have settled it yet, as `detach` settles it.
            self.settle();

            return;
        }
        // Marked done first, so that the guard in the wrapper finds nobody left to tell when the
        // future goes, here or after the poll under way.
        let join_waker = {
            let mut state = self.join.state.lock();
            // Finished, or failed as its runtime went: nothing to take back. An output nobody
            // collected is let go of here all the same, on the thread that let the handle go,
            // rather than left to whoever drops the last clone of the task's waker, which on a
            // shared runtime holds the output alongside it.
            if state.done {
                let settled = (state.output.take(), state.join_waker.take());
                drop(state);
                // Clear of the lock: an output's destructor is the task's code, and a waker's
                // somebody else's.
                drop(settled);

                return;
            }
            state.done = true;

            state.join_waker.take()
        };
        self.stop(join_waker);
    }
}

impl<T, M> fmt::Debug for JoinHandle<T, M>
where
    M: Mode,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JoinHandle")
            .field("task", &self.join.name)
            .field("detached", &self.detached)
            .finish_non_exhaustive()
    }
}

/// What a task and its handle share: the task's name, its waker's state, and the outcome the
/// handle waits on.
pub(crate) struct Join<M, T>
where
    M: Mode,
{
    /// What the task is called: the name its spawner gave it, or where it was spawned, for a
    /// spawn that took none. It is for diagnostics alone: the message a panicking task logs, and
    /// the handle's [`Debug`](fmt::Debug).
    name: Name,
    /// Behind an `Arc` of its own on a local runtime, and in place on a shared one, where this is
    /// what the task's waker points at.
    waker: M::HeldWaker,
    state: M::Lock<JoinState<T>>,
}

impl<M, T> Join<M, T>
where
    M: Mode,
{
    /// The state of the task's waker.
    fn task_waker(&self) -> &TaskWaker {
        self.waker.borrow()
    }
}

/// The waker of a task on a shared runtime: what the task and its handle share, which is `Send`
/// and `Sync` there, so that a spawn allocates no waker of its own.
///
/// A waker somebody keeps keeps all of that alive with it, output included where the task has
/// one and nobody took it; a handle that is detached or dropped settles the outcome, so that the
/// task drops its output as it hands it over instead.
impl<T> Wake for Join<Shared, T>
where
    T: Send + 'static,
{
    fn wake(self: Arc<Self>) {
        self.waker.wake();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.waker.wake();
    }
}

/// What a task is called in diagnostics: the name its spawner gave it, or, where it was given none,
/// the place it was spawned at.
///
/// Holds nothing it had to allocate for, so that a spawn without a name costs no more than one
/// with a name that is a literal.
pub(crate) enum Name {
    /// A name the spawner chose, which says what the task is there for.
    Given(Cow<'static, str>),
    /// The call that spawned the task, for a spawn that took no name.
    SpawnedAt(&'static Location<'static>),
}

/// How the log names a task: the given name between backticks, or where the task was spawned.
impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Given(name) => write!(f, "`{name}`"),
            Self::SpawnedAt(location) => write!(f, "spawned at {location}"),
        }
    }
}

/// A given name prints as the string it is. The place a task was spawned at prints without quotes
/// around it, as `Location` prints itself in a panic message, so that a path with backslashes in
/// it is not escaped out of recognition.
impl fmt::Debug for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Given(name) => fmt::Debug::fmt(name, f),
            Self::SpawnedAt(_) => fmt::Display::fmt(self, f),
        }
    }
}

/// The outcome of a task, and the waker of whoever is joining it.
struct JoinState<T> {
    /// The task's output, until the handle that joins it takes it.
    output: Option<io::Result<T>>,
    /// Whether the task's outcome is settled: an output handed over, or the handle gone and so
    /// nobody left to hand one to. A handle that cancels the task and stays leaves it for the task
    /// to settle, which the task does once its future is gone, so that the handle learns when
    /// that is.
    done: bool,
    /// Registered by a poll of the [`JoinHandle`] that found no output waiting yet.
    join_waker: Option<Waker>,
}

/// Hands the outcome of a task over to its handle: the one it is given, or, where it is dropped
/// with none given, the error of a task whose runtime went away with it.
///
/// Dropped unfinished, a task's wrapper is cancelled, or dropped with the runtime it was on. A
/// handle that went as it cancelled the task marked the outcome settled already, and there is
/// nobody to tell. One that cancelled the task and stayed is waiting to hear that the future is
/// gone, which the error tells it, though it hands that error on to nobody. Otherwise the runtime
/// went, which is what the error says.
struct CompleteOnDrop<M, T>(M::Ptr<Join<M, T>>)
where
    M: Mode;

impl<M, T> CompleteOnDrop<M, T>
where
    M: Mode,
{
    /// Hands `output` over, unless the handle is gone and has nobody to hand it to.
    fn complete(&self, output: io::Result<T>) {
        let (unwanted, join_waker) = {
            let mut state = self.0.state.lock();
            if state.done {
                (Some(output), None)
            } else {
                state.done = true;
                state.output = Some(output);

                (None, state.join_waker.take())
            }
        };
        // Clear of the lock: an output's destructor is the task's code.
        drop(unwanted);
        if let Some(waker) = join_waker {
            // Outside the lock: the handle may be polled to completion on this very thread.
            waker.wake();
        }
    }
}

impl<M, T> Drop for CompleteOnDrop<M, T>
where
    M: Mode,
{
    fn drop(&mut self) {
        let join_waker = {
            let mut state = self.0.state.lock();
            if state.done {
                return;
            }
            state.done = true;
            state.output = Some(Err(io::Error::other("the task's runtime is gone")));

            state.join_waker.take()
        };
        if let Some(waker) = join_waker {
            waker.wake();
        }
    }
}

/// Polls the future in `future` until it is done, with every panic a poll of it raises caught
/// and handed back in place of its output, marking the task `task_waker` wakes unqueued before
/// each poll.
///
/// The future sits in an `Option` so that the caller can drop it once this is done, in place and
/// with its own panics caught, before anybody hears of the outcome.
fn catch_unwind_polls<'a, F>(
    task_waker: &'a TaskWaker,
    mut future: Pin<&'a mut Option<F>>,
) -> impl Future<Output = thread::Result<F::Output>> + 'a
where
    F: Future + 'a,
{
    poll_fn(move |cx| {
        // The queue entry this poll was taken off is spent by this: a wake that arrives from here
        // on, the poll below included, queues the task afresh. A swap rather than a store, so
        // that the poll sees what the wakes turned away since the entry was queued were for.
        // Here rather than in the scheduler, which holds the task's waker only as a `Waker`.
        task_waker.queued.swap(false, Ordering::AcqRel);
        let Some(future) = future.as_mut().as_pin_mut() else {
            unreachable!("the future is polled until it is done, and dropped only then");
        };
        match catch_unwind(AssertUnwindSafe(|| future.poll(cx))) {
            Ok(Poll::Ready(value)) => Poll::Ready(Ok(value)),
            Ok(Poll::Pending) => Poll::Pending,
            Err(payload) => Poll::Ready(Err(payload)),
        }
    })
}

/// Drops `value` with every panic its destructor raises contained.
///
/// A task's panic is contained per task: the destructor of its payload, and of that payload's
/// own payload should it have one, is part of that same task rather than of whoever polls it.
fn dispose<T>(value: T) {
    let Err(payload) = catch_unwind(AssertUnwindSafe(move || drop(value))) else {
        return;
    };
    // A payload whose own destructor panics is leaked rather than dropped a third time: the
    // panic is contained either way, and a chain of destructors that each panic with the next
    // is not one the scheduler should follow to its end.
    if let Err(payload) = catch_unwind(AssertUnwindSafe(move || drop(payload))) {
        mem::forget(payload);
    }
}

/// The tasks spawned on a scheduler that have not ended yet, under their ids.
struct Tasks<M>
where
    M: Mode,
{
    /// Keyed, rather than a list to scan: a server handing each request a task of its own would
    /// make a scan per completion quadratic under a burst of them.
    slots: HashMap<u64, Slot<M>, BuildHasherDefault<IdHasher>>,
    /// The id the next task gets. Never handed out twice, so that a stale id in the ready queue
    /// can only ever name a task that is gone, never a newer one in its place.
    next_id: u64,
}

/// Hashes a task id as itself.
///
/// Ids come from a counter, so they are spread as evenly as a hash could spread them, and the
/// default hasher's defence against chosen keys buys nothing for keys no caller chooses.
#[derive(Default)]
struct IdHasher(u64);

impl Hasher for IdHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 = (self.0 << 8) | u64::from(*byte);
        }
    }

    fn write_u64(&mut self, id: u64) {
        self.0 = id;
    }
}

/// One task, and what it is up to.
enum Slot<M>
where
    M: Mode,
{
    /// Waiting to be polled, with its future in hand, and its waker, made once as it was spawned:
    /// both are moved out for a poll and back after it, so a poll clones nothing.
    Idle { future: M::BoxFuture, waker: Waker },
    /// Out of its slot, being polled.
    Running {
        /// Whether the handle was dropped, or cancelled the task, while the task was being
        /// polled, so the future is to be dropped once that poll returns.
        cancelled: bool,
    },
}

/// The state of one task's waker: its id, and the queue it goes on when woken.
///
/// Holds nothing of the task's future, which may not leave the thread its runtime is on, so that
/// it can be woken from any thread whatever the runtime's flavour.
///
/// Public in name only, in a module nobody outside can reach, because the sealed trait behind
/// [`Mode`] names it.
pub struct TaskWaker {
    id: u64,
    /// Whether the task's id sits in the ready queue, so that a wake which finds it there leaves
    /// it at that. Cleared right before the task's own future is polled.
    queued: AtomicBool,
    remote: Arc<Remote>,
}

impl TaskWaker {
    /// Queues the task, unless it is queued already.
    fn wake(&self) {
        // A previous `true` means the task is queued already, and the poll that entry leads to
        // sees what this wake was for: it acquires this very swap as it clears the flag.
        if self.queued.swap(true, Ordering::AcqRel) {
            return;
        }
        self.remote.schedule(self.id);
    }
}

/// The waker of a task on a local runtime, whose shared state is not `Sync` and so cannot be
/// what the waker points at.
impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        TaskWaker::wake(&self);
    }

    fn wake_by_ref(self: &Arc<Self>) {
        TaskWaker::wake(self);
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "event")]
    use std::thread;
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
        sync::atomic::{AtomicBool, Ordering},
        time::Duration,
    };

    use futures::executor::block_on;
    use ntest::timeout;

    use super::*;
    #[cfg(feature = "event")]
    use crate::Event;
    use crate::{LocalRuntime, Task, tests::yield_now};

    #[test]
    #[timeout(15000)]
    fn a_spawned_task_runs_and_hands_its_output_back() {
        let runtime = runtime();
        let handle = runtime.spawn("an answer", async { 42 });

        drive(&runtime);

        assert_eq!(block_on(handle).unwrap(), 42);
        assert_eq!(runtime.core.scheduler.live_tasks(), 0);
    }

    #[cfg(feature = "event")]
    #[test]
    #[timeout(15000)]
    fn a_wake_from_another_thread_notifies_the_driver() {
        let runtime = runtime();
        let event = Arc::new(Event::new());
        let handle = {
            let event = event.clone();
            runtime.spawn("a waiting task", async move {
                event.listen().await;
                "woken"
            })
        };

        // Polled here, so that it is waiting for the event rather than for its first poll.
        drive(&runtime);
        assert!(!runtime.core.remote.has_ready());
        take_wake_up(&runtime);

        thread::spawn(move || {
            event.notify(1);
        })
        .join()
        .unwrap();

        assert!(runtime.core.remote.wake_pending());
        assert!(runtime.core.remote.has_ready());
        drive(&runtime);
        assert_eq!(block_on(handle).unwrap(), "woken");
    }

    #[test]
    #[timeout(15000)]
    fn a_task_woken_during_its_own_poll_runs_again() {
        struct WakeOnce(bool);

        impl Future for WakeOnce {
            type Output = ();

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
                if self.0 {
                    return Poll::Ready(());
                }
                self.0 = true;
                cx.waker().wake_by_ref();

                Poll::Pending
            }
        }

        let runtime = runtime();
        let scheduler = &runtime.core.scheduler;
        let handle = runtime.spawn("a self-waking task", WakeOnce(false));

        assert!(scheduler.run_one());
        assert!(runtime.core.remote.has_ready());
        assert!(scheduler.run_one());
        assert!(!scheduler.run_one());
        assert!(block_on(handle).is_ok());
    }

    /// A task that a poll of a batch woke runs again in that batch, budget permitting, rather
    /// than after the wait on the reactor that follows it.
    #[test]
    #[timeout(15000)]
    fn a_task_woken_during_a_batch_runs_again_in_it() {
        let runtime = runtime();
        let scheduler = &runtime.core.scheduler;
        let handle = runtime.spawn("a task that yields once", async {
            yield_now().await;
            5
        });

        // A budget of one poll leaves the task queued again once that poll woke it.
        scheduler.run_batch(&mut [0; 1]);
        assert_eq!(runtime.core.remote.ready_len(), 1);
        scheduler.run_batch(&mut [0; 1]);
        assert_eq!(scheduler.live_tasks(), 0);

        let handle_2 = runtime.spawn("another task that yields once", async {
            yield_now().await;
            6
        });
        scheduler.run_batch(&mut [0; 2]);

        assert_eq!(scheduler.live_tasks(), 0);
        assert!(!runtime.core.remote.has_ready());
        assert_eq!(block_on(handle).unwrap(), 5);
        assert_eq!(block_on(handle_2).unwrap(), 6);
    }

    #[test]
    #[timeout(15000)]
    fn dropping_the_handle_cancels_and_drops_the_future() {
        let runtime = runtime();
        let dropped = Arc::new(AtomicBool::new(false));
        let handle = {
            let marker = SetOnDrop(dropped.clone());
            runtime.spawn("a task that never finishes", async move {
                let _marker = marker;
                std::future::pending::<()>().await;
            })
        };

        // Polled here, so that the cancellation finds it idle rather than merely queued.
        drive(&runtime);
        assert!(!dropped.load(Ordering::Acquire));

        drop(handle);

        assert!(dropped.load(Ordering::Acquire));
        assert_eq!(runtime.core.scheduler.live_tasks(), 0);
    }

    /// A task cancelled while it waits in the queue leaves its id behind, which is skipped.
    ///
    /// The id counts as one polled, so that a batch taking it moves on, and nothing is polled for
    /// it: the task it named is gone, and ids are never handed out again, so no newer task can
    /// have taken its place.
    #[test]
    #[timeout(15000)]
    fn a_task_cancelled_while_queued_leaves_an_id_that_is_skipped() {
        let runtime = runtime();
        let polled = Rc::new(Cell::new(false));
        let handle = {
            let polled = polled.clone();
            runtime.spawn("a task that is never polled", async move {
                polled.set(true);
            })
        };

        drop(handle);

        assert_eq!(runtime.core.remote.ready_len(), 1);
        assert!(runtime.core.scheduler.run_one());
        assert!(!runtime.core.scheduler.run_one());
        assert!(!polled.get());
    }

    #[test]
    #[timeout(15000)]
    fn cancelling_notifies_the_driver() {
        let runtime = runtime();
        let handle = runtime.spawn("a task that never finishes", std::future::pending::<()>());

        drive(&runtime);
        take_wake_up(&runtime);

        drop(handle);

        assert!(runtime.core.remote.wake_pending());
    }

    #[test]
    #[timeout(15000)]
    fn a_detached_task_runs_to_completion() {
        let runtime = runtime();
        let ran = Arc::new(AtomicBool::new(false));
        {
            let ran = ran.clone();
            runtime
                .spawn("a detached task", async move {
                    yield_now().await;
                    ran.store(true, Ordering::Release);
                })
                .detach();
        }

        drive(&runtime);

        assert!(ran.load(Ordering::Acquire));
        assert_eq!(runtime.core.scheduler.live_tasks(), 0);
    }

    #[test]
    #[timeout(15000)]
    fn a_panicking_task_fails_its_handle_and_is_forgotten() {
        let runtime = runtime();
        let handle = runtime.spawn("a task that panics", async {
            panic!("the task panicked on purpose");
        });

        drive(&runtime);

        let error = block_on(handle).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::Other);
        assert_eq!(runtime.core.scheduler.live_tasks(), 0);
    }

    /// A panic whose payload panics as it goes takes its task down with it all the same.
    ///
    /// The payload is the task's to dispose of, and a destructor that panics is one more panic
    /// of that task rather than one of the thread polling it. The task is finished with either
    /// way: nobody will poll it again, and whoever waits on its handle is told so.
    #[test]
    #[timeout(15000)]
    fn a_panic_payload_that_panics_as_it_goes_still_ends_its_task() {
        struct PanickingPayload;

        impl Drop for PanickingPayload {
            fn drop(&mut self) {
                panic!("the panic payload's drop panicked on purpose");
            }
        }

        let runtime = runtime();
        let scheduler = &runtime.core.scheduler;
        let handle = runtime.spawn("a task that panics with a payload of its own", async {
            std::panic::panic_any(PanickingPayload);
        });

        assert!(scheduler.run_one());

        assert_eq!(scheduler.live_tasks(), 0);
        assert_eq!(block_on(handle).unwrap_err().kind(), io::ErrorKind::Other);
        assert!(!scheduler.run_one());
    }

    /// Cancelling a task from inside its own poll, whose future's drop panics with a payload
    /// whose own drop panics too, is contained at every level: the scheduler runs the next task
    /// instead of unwinding past the one that was cancelled.
    #[test]
    #[timeout(15000)]
    fn cancelling_a_task_whose_drop_panics_with_a_panicking_payload_is_contained() {
        struct PanickingPayload;

        impl Drop for PanickingPayload {
            fn drop(&mut self) {
                panic!("the payload's drop panicked on purpose");
            }
        }

        struct PanicsWithAPayloadWhenDropped;

        impl Drop for PanicsWithAPayloadWhenDropped {
            fn drop(&mut self) {
                std::panic::panic_any(PanickingPayload);
            }
        }

        let runtime = runtime();
        let scheduler = &runtime.core.scheduler;
        let own_handle: Rc<RefCell<Option<Task<()>>>> = Rc::new(RefCell::new(None));
        let dropped_from_inside = own_handle.clone();
        let handle = runtime.spawn(
            "a task that cancels itself from within its own poll",
            async move {
                let _marker = PanicsWithAPayloadWhenDropped;
                dropped_from_inside.borrow_mut().take();
                std::future::pending::<()>().await;
            },
        );
        *own_handle.borrow_mut() = Some(handle);

        let ran = Arc::new(AtomicBool::new(false));
        let flag = ran.clone();
        let other = runtime.spawn("an unrelated task", async move {
            flag.store(true, Ordering::Release);
        });

        assert!(scheduler.run_one());
        assert!(scheduler.run_one());

        assert!(ran.load(Ordering::Acquire));
        assert!(block_on(other).is_ok());
        assert_eq!(scheduler.live_tasks(), 0);
    }

    /// A panic payload whose own drop panics with a further payload, whose drop panics too, is
    /// contained at every level: the task is failed and the next one still runs.
    #[test]
    #[timeout(15000)]
    fn a_panic_payload_whose_drop_panics_with_another_payload_is_contained() {
        struct Inner;

        impl Drop for Inner {
            fn drop(&mut self) {
                panic!("the inner payload's drop panicked on purpose");
            }
        }

        struct Outer;

        impl Drop for Outer {
            fn drop(&mut self) {
                std::panic::panic_any(Inner);
            }
        }

        let runtime = runtime();
        let scheduler = &runtime.core.scheduler;
        let handle = runtime.spawn("a task whose panic payload panics twice over", async {
            std::panic::panic_any(Outer);
        });

        assert!(scheduler.run_one());

        assert_eq!(scheduler.live_tasks(), 0);
        assert_eq!(block_on(handle).unwrap_err().kind(), io::ErrorKind::Other);

        let next = runtime.spawn("the task after it", async { 7 });
        drive(&runtime);
        assert_eq!(block_on(next).unwrap(), 7);
    }

    #[test]
    #[timeout(15000)]
    fn a_panic_in_a_futures_drop_is_contained() {
        struct PanicOnDrop;

        impl Future for PanicOnDrop {
            type Output = ();

            fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
                Poll::Ready(())
            }
        }

        impl Drop for PanicOnDrop {
            fn drop(&mut self) {
                panic!("the future's drop panicked on purpose");
            }
        }

        let runtime = runtime();
        let handle = runtime.spawn("a task that panics as it goes", PanicOnDrop);

        assert!(runtime.core.scheduler.run_one());
        assert!(block_on(handle).is_ok());

        let next = runtime.spawn("the task after it", async { 7 });
        drive(&runtime);
        assert_eq!(block_on(next).unwrap(), 7);
    }

    /// A cancellation whose future panics as it goes notifies all the same.
    ///
    /// The notification is what tells a thread waiting with this as its last task that it has
    /// nothing left to wait for, and a destructor of somebody else's that panicked is no reason
    /// to leave that thread waiting.
    #[test]
    #[timeout(15000)]
    fn a_panic_in_a_cancelled_futures_drop_still_notifies() {
        struct PanicOnDrop;

        impl Drop for PanicOnDrop {
            fn drop(&mut self) {
                panic!("the future's drop panicked on purpose");
            }
        }

        let runtime = runtime();
        let handle = {
            let marker = PanicOnDrop;
            runtime.spawn("a task that never finishes", async move {
                let _marker = marker;
                std::future::pending::<()>().await;
            })
        };
        // Polled here, so that the cancellation finds the future idle and the drop of it is this
        // thread's to make.
        drive(&runtime);
        take_wake_up(&runtime);

        let cancelled = catch_unwind(AssertUnwindSafe(|| drop(handle)));

        // The panic is the caller's to see, where the handle was let go of.
        assert!(cancelled.is_err());
        assert!(runtime.core.remote.wake_pending());
        assert_eq!(runtime.core.scheduler.live_tasks(), 0);
    }

    #[test]
    #[timeout(15000)]
    fn a_ready_task_is_queued_once_however_often_it_is_woken() {
        let runtime = runtime();
        let scheduler = &runtime.core.scheduler;
        let handle = runtime.spawn("a task that never finishes", std::future::pending::<()>());
        let waker = scheduler.waker(handle.0.id);

        for _ in 0..3 {
            waker.wake_by_ref();
        }

        assert_eq!(runtime.core.remote.ready_len(), 1);
        assert!(scheduler.run_one());
        assert!(!scheduler.run_one());
    }

    #[test]
    #[timeout(15000)]
    fn a_cancelled_futures_drop_may_spawn_on_the_scheduler() {
        struct SpawnOnDrop {
            runtime: LocalRuntime,
            ran: Rc<Cell<bool>>,
        }

        impl Drop for SpawnOnDrop {
            fn drop(&mut self) {
                let ran = self.ran.clone();
                self.runtime
                    .spawn("a task spawned from a drop", async move {
                        ran.set(true);
                    })
                    .detach();
            }
        }

        let runtime = runtime();
        let ran = Rc::new(Cell::new(false));
        let handle = {
            let spawner = SpawnOnDrop {
                runtime: runtime.clone(),
                ran: ran.clone(),
            };
            runtime.spawn("a task that never finishes", async move {
                let _spawner = spawner;
                std::future::pending::<()>().await;
            })
        };

        drive(&runtime);
        drop(handle);

        assert!(!ran.get());
        drive(&runtime);
        assert!(ran.get());
    }

    #[test]
    #[timeout(15000)]
    fn live_tasks_counts_unfinished_futures_only() {
        let runtime = runtime();
        let done = runtime.spawn("a task that finishes at once", async {});
        let pending = runtime.spawn("a task that never finishes", std::future::pending::<()>());
        assert_eq!(runtime.core.scheduler.live_tasks(), 2);

        drive(&runtime);

        assert_eq!(runtime.core.scheduler.live_tasks(), 1);
        assert!(block_on(done).is_ok());

        drop(pending);

        assert_eq!(runtime.core.scheduler.live_tasks(), 0);
    }

    #[test]
    #[timeout(15000)]
    fn dropping_the_runtime_fails_pending_joins() {
        let runtime = runtime();
        let handle = runtime.spawn("a task that never finishes", std::future::pending::<()>());

        drive(&runtime);
        drop(runtime);

        assert!(block_on(handle).is_err());
    }

    /// A task dropped with its runtime drops its future before its handle hears of it, whether
    /// it was polled before the runtime went or never was: whoever joins it finds whatever the
    /// future held already gone, as they do where the task ran to its end.
    #[test]
    #[timeout(15000)]
    fn a_task_dropped_with_its_runtime_drops_its_future_before_failing_its_handle() {
        for polled in [false, true] {
            let runtime = runtime();
            let handle = Rc::new(RefCell::new(None));
            let outcome_seen = Rc::new(Cell::new(None));
            let probe = ProbeOnDrop {
                handle: handle.clone(),
                outcome_seen: outcome_seen.clone(),
            };
            let task = runtime.spawn("a task that never finishes", async move {
                let _probe = probe;
                std::future::pending::<()>().await;
            });
            *handle.borrow_mut() = Some(task);
            if polled {
                drive(&runtime);
            }

            drop(runtime);

            assert_eq!(outcome_seen.get(), Some(false), "polled: {polled}");
            let task = handle.borrow_mut().take().unwrap();
            assert!(block_on(task).is_err());
        }
    }

    /// A name a spawner gave reads as it always has, and the place a task was spawned at, for one
    /// that was given none, reads as a place: in its `Debug`, which a handle's prints, and in the
    /// sentence the log line of a panicking task puts it in, written out here as that `error!`
    /// writes it. No test sees the line logged itself.
    #[test]
    #[timeout(15000)]
    fn a_name_reads_naturally_in_the_log_and_in_debug() {
        let given = Name::Given("socket reader".into());
        let location = Location::caller();
        let spawned_at = Name::SpawnedAt(location);

        assert_eq!(
            format!("The task {given} panicked"),
            "The task `socket reader` panicked"
        );
        assert_eq!(format!("{given:?}"), r#""socket reader""#);
        assert_eq!(
            format!("The task {spawned_at} panicked"),
            format!("The task spawned at {location} panicked")
        );
        assert_eq!(format!("{spawned_at:?}"), format!("spawned at {location}"));
        assert!(location.file().ends_with("scheduler.rs"));
    }

    /// Polls ready tasks until none is left, the way the thread driving the runtime does.
    fn drive(runtime: &LocalRuntime) {
        while runtime.core.scheduler.run_one() {}
    }

    /// Takes the wake-up the runtime has written, if any, out of its channel, so that the next
    /// one a test looks for is one written after this.
    fn take_wake_up(runtime: &LocalRuntime) {
        runtime.core.reactor.wait(Some(Duration::ZERO)).unwrap();
        assert!(!runtime.core.remote.wake_pending());
    }

    /// A runtime of the test's own, driven by nobody but the test.
    fn runtime() -> LocalRuntime {
        LocalRuntime::new().unwrap()
    }

    /// Sets the flag it holds when it is dropped.
    struct SetOnDrop(Arc<AtomicBool>);

    impl Drop for SetOnDrop {
        fn drop(&mut self) {
            self.0.store(true, Ordering::Release);
        }
    }

    /// Polls the task in `handle` once as it is dropped, and notes in `outcome_seen` whether
    /// that task had an outcome for its handle yet.
    struct ProbeOnDrop {
        handle: Rc<RefCell<Option<Task<()>>>>,
        outcome_seen: Rc<Cell<Option<bool>>>,
    }

    impl Drop for ProbeOnDrop {
        fn drop(&mut self) {
            let mut handle = self.handle.borrow_mut();
            let Some(task) = handle.as_mut() else {
                return;
            };
            let polled = Pin::new(task).poll(&mut Context::from_waker(Waker::noop()));
            self.outcome_seen.set(Some(polled.is_ready()));
        }
    }
}
