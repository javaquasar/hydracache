//! What watches a runtime's sockets and keeps its timers.
//!
//! A reactor holds every source registered on it and every timer taken from it, and the thread
//! driving the runtime hands it that thread through [`Reactor::wait`]: one wait on the platform's
//! poller, bounded by the nearest deadline, and then the wakes for whatever that wait found ready
//! and for whatever timer has come due.
//!
//! The poller watches each source in the directions the reactor last told it to, and is told of a
//! change rather than handed every source on each wait, so that a wait costs what the sources
//! found ready cost and no more, where the platform's poller allows. The poll is level-triggered:
//! a source watched in a direction it is ready in is reported by every wait until it is not. A
//! direction is watched from the moment somebody waits in it: the waiter that stores the first
//! waker of a direction has the poller told there and then, and breaks the wait under way where
//! the platform's poller sees the change only in the wait after it. A direction nobody waits in
//! any more is let go of later, before the next wait, and only where nobody has come back to wait
//! in it again by then: a task woken by readiness usually stores its waker again as it runs out
//! of what to read, and that costs the poller nothing. A source a wait found ready, or one a
//! waiter gave up on, is listed for that look before the next wait.
//!
//! Two locks guard the sources and the timers, and neither is taken while the other is held. A
//! source's wakers live in its entry in the map, under the map's lock, and every change the poller
//! is told of is made under that lock too, so that two of them never reach it out of order. Neither
//! lock is held across the wait, which may last until a deadline, nor across a wake or the drop of
//! a waker or of the last clone of a source, each of which runs somebody else's code and may come
//! straight back here to register a source, to ask for a timer or to let either go. The flag that
//! spares a `notify` its write while a wake-up is on its way needs no lock: it is an atomic of the
//! runtime's remote.

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt,
    future::Future,
    io, mem,
    pin::Pin,
    sync::{Arc, atomic::Ordering},
    task::{Context, Poll, Waker, ready},
    time::{Duration, Instant},
};

use crate::{
    Interest, Local, Mode, coop,
    log::error,
    mode::sealed::Lock,
    poll::{self, Directions, Poller, RawSource},
    runtime::{Core, Remote},
};

/// The sockets one runtime watches and the timers it keeps.
pub(crate) struct Reactor<M>
where
    M: Mode,
{
    sources: M::Lock<Sources<M>>,
    timers: M::Lock<Timers>,
    /// The poller a wait is made on, and the flag that says a wake-up is on its way to it.
    remote: Arc<Remote>,
}

impl<M> Reactor<M>
where
    M: Mode,
{
    /// A reactor watching nothing, which waits on `remote`'s poller.
    pub(crate) fn new(remote: Arc<Remote>) -> Self {
        Self {
            sources: Lock::new(Sources {
                states: HashMap::new(),
                descriptors: HashSet::new(),
                next_key: 0,
                next_waiter: 0,
                stale: Vec::new(),
                waiting: false,
                retired: Vec::new(),
            }),
            timers: Lock::new(Timers::default()),
            remote,
        }
    }

    /// One wait on the poller, bounded by `at_most` and by the nearest deadline, then the wakes
    /// for what it found ready and for the timers that are due.
    pub(crate) fn wait(&self, at_most: Option<Duration>) -> io::Result<()> {
        {
            let mut sources = self.sources.lock();
            sources.reconcile(&self.remote.poller)?;
            // A poller that copies its list as a wait starts watches, until the wait returns,
            // descriptors whose registrations may go in the meantime: those are kept open until
            // then, which `Registration::drop` reads this for.
            sources.waiting = !Poller::LIVE;
        }
        let timeout = self.timeout(at_most);

        let ready = match self.remote.poller.wait(timeout) {
            Ok(ready) => ready,
            Err(e) => {
                self.end_wait();

                return Err(e);
            }
        };
        // The wait takes whatever wake-up it finds out of the channel, so the flag comes down
        // here and the next caller writes again. One that came between the two is turned away
        // without a write, and loses nothing by it: what it had to say — a task queued, a source
        // registered, a deadline stored — it said before it called, and the rounds below and the
        // ones the driving thread makes look at all three afresh. The other way about, a wake-up
        // left in the channel by a wait that ended some other way only ends the next one at once.
        self.remote.wake_pending.store(false, Ordering::Release);

        let (woken, retired) = {
            let mut sources = self.sources.lock();
            sources.waiting = false;
            let Sources {
                states,
                stale,
                retired,
                ..
            } = &mut *sources;
            let mut woken = Vec::new();
            for event in &ready {
                // A source let go of while the wait ran has nobody left to wake.
                let Some(state) = states.get_mut(&event.key) else {
                    continue;
                };
                if event.directions.readable {
                    state.wakers.readable.take_into(&mut woken);
                }
                if event.directions.writable {
                    state.wakers.writable.take_into(&mut woken);
                }
                // A direction the poller let go of by itself is watched no more: the next waiter
                // in it has the poller watch it again.
                state.armed.readable &= !event.dropped.readable;
                state.armed.writable &= !event.dropped.writable;
                // Watched still in what it was found ready in, which nobody may wait in any
                // more: looked at again before the next wait.
                state.mark_stale(event.key, stale);
            }

            (woken, mem::take(retired))
        };
        // Clear of every lock: the last clone of a source whose registration went while the wait
        // ran closes the source, which runs the destructor of whatever was registered.
        drop(retired);
        for waker in woken {
            waker.wake();
        }

        let due = {
            let mut timers = self.timers.lock();
            // Ids are handed out from zero upwards, so no timer carries `u64::MAX` and the split
            // leaves behind exactly the deadlines that have passed.
            let later = timers.pending.split_off(&(Instant::now(), u64::MAX));

            mem::replace(&mut timers.pending, later)
        };
        for waker in due.into_values() {
            waker.wake();
        }

        Ok(())
    }

    /// How long a wait may last: no longer than `at_most`, than until the nearest deadline, or
    /// than [`poll::MAX_TIMEOUT`], and without limit where none of those bounds it.
    fn timeout(&self, at_most: Option<Duration>) -> Option<Duration> {
        let deadline = self
            .timers
            .lock()
            .pending
            .keys()
            .next()
            .map(|(deadline, _)| *deadline);
        let until_deadline = deadline.map(|at| at.saturating_duration_since(Instant::now()));
        let timeout = match (at_most, until_deadline) {
            (Some(at_most), Some(until_deadline)) => Some(at_most.min(until_deadline)),
            (bound, None) | (None, bound) => bound,
        };

        timeout.map(|timeout| timeout.min(poll::MAX_TIMEOUT))
    }

    /// Ends a wait that failed: lets go, clear of every lock, of the sources whose registrations
    /// went while it ran. A wait that succeeds does the same under the lock it takes to wake the
    /// waiters of what it found ready.
    ///
    /// The drop of the last clone of a source closes it, which runs the destructor of whatever
    /// was registered: somebody else's code, which may come straight back here.
    fn end_wait(&self) {
        let retired = {
            let mut sources = self.sources.lock();
            sources.waiting = false;

            mem::take(&mut sources.retired)
        };
        drop(retired);
    }

    /// Wakes every stored waker, sources and timers alike; what a failed wait falls back on, so
    /// that each waiter retries its operation and sees its own error.
    pub(crate) fn wake_everything(&self) {
        let mut woken = Vec::new();
        {
            let mut sources = self.sources.lock();
            let Sources { states, stale, .. } = &mut *sources;
            for (&key, state) in states.iter_mut() {
                state.wakers.readable.take_into(&mut woken);
                state.wakers.writable.take_into(&mut woken);
                // Watched still for the waiters just woken, who may not wait again.
                state.mark_stale(key, stale);
            }
        }
        woken.extend(mem::take(&mut self.timers.lock().pending).into_values());

        for waker in woken {
            waker.wake();
        }
    }

    /// No registered source and no pending timer.
    #[cfg(any(test, feature = "helper"))]
    pub(crate) fn is_idle(&self) -> bool {
        let no_sources = self.sources.lock().states.is_empty();

        no_sources && self.timers.lock().pending.is_empty()
    }
}

/// Puts `source` under the watch of `core`'s reactor, with a key of its own to report it by.
pub(crate) fn register<M>(
    core: &M::Ptr<Core<M>>,
    source: M::SourcePtr,
) -> io::Result<Registration<M>>
where
    M: Mode,
{
    // Read here, once, and never again: a wait watches the descriptor the source lends now, so
    // that it runs no code of the source's. Clear of the lock, as that is somebody else's code.
    #[cfg(any(unix, target_os = "wasi"))]
    let descriptor = std::os::fd::AsRawFd::as_raw_fd(&M::as_source(&source));
    #[cfg(windows)]
    let descriptor = std::os::windows::io::AsRawSocket::as_raw_socket(&M::as_source(&source));
    let key = {
        let mut sources = core.reactor.sources.lock();
        // One descriptor under two keys is more than some of the systems' pollers take: epoll
        // turns the second away, and kqueue would have it take the place of the first. A runtime
        // turns it away on every platform, so that a program does on each what it does on any.
        if !sources.descriptors.insert(descriptor) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "this runtime watches the source's descriptor already",
            ));
        }
        let key = sources.next_key;
        if let Err(e) = core.remote.poller.add(key, descriptor) {
            sources.descriptors.remove(&descriptor);

            return Err(e);
        }
        sources.next_key += 1;
        sources.states.insert(
            key,
            SourceState {
                source,
                descriptor,
                wakers: Wakers::default(),
                armed: Directions::NONE,
                stale: false,
            },
        );

        key
    };

    Ok(Registration {
        core: core.clone(),
        key,
    })
}

/// A timer of `core`'s reactor that comes due at `deadline`, or one that never comes due where
/// there is none.
pub(crate) fn sleep<M>(core: &M::Ptr<Core<M>>, deadline: Option<Instant>) -> Sleep<M>
where
    M: Mode,
{
    Sleep {
        core: core.clone(),
        deadline,
        id: None,
    }
}

/// A source that a [`Runtime`](crate::Runtime) watches, and the handle to do its I/O through.
///
/// A registration holds its runtime, so a task that holds one keeps the runtime alive.
pub struct Registration<M = Local>
where
    M: Mode,
{
    core: M::Ptr<Core<M>>,
    key: usize,
}

impl<M> fmt::Debug for Registration<M>
where
    M: Mode,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Registration").finish_non_exhaustive()
    }
}

impl<M> Registration<M>
where
    M: Mode,
{
    /// Runs `operation` while the source of this registration is ready for `interest`, and retries
    /// it when new readiness arrives.
    ///
    /// Returns the first success of `operation`, a partial write included, or its first error other
    /// than [`WouldBlock`](io::ErrorKind::WouldBlock). A `WouldBlock` means the source was not
    /// ready after all. In that case, this arranges for the waker of `cx` to be woken once it is,
    /// and returns [`Poll::Pending`]. If the OS then refuses to watch the source for `interest`,
    /// this returns that error instead.
    ///
    /// A registration keeps one waker per interest for this, so only one operation at a time can
    /// wait in each direction. If a second operation waits to read, for example, it replaces the
    /// first one, which is then never woken. One reader and one writer can wait together. More of
    /// either must take turns, for example behind a lock, or wait through
    /// [`ready`](Registration::ready), which any number of tasks can do at once, and run their
    /// operation once it completes.
    ///
    /// This never spins while the source is not ready. Since the source is non-blocking, as
    /// [`Runtime::register`](crate::Runtime#method.register) requires, it never blocks the thread
    /// either.
    ///
    /// Nor does it let a source that is always ready keep the thread from the other tasks. A poll
    /// of a task, or of the future passed to `block_on`, may complete 128 I/O operations
    /// through this and through [`AsyncIo`](crate::AsyncIo), which counts them the same way.
    /// The next one yields, and so does every one after it until the poll is over: this wakes the
    /// task itself and returns [`Poll::Pending`] without running `operation`, which sends the task
    /// to the back of the queue. The tasks queued before it have their turn then, and the reactor
    /// has its own once the batch of polls the task is in is over. An `operation` that fails with
    /// `WouldBlock` is not counted.
    ///
    /// The budget belongs to the thread, not to the future. Whatever is polled inside the poll of
    /// a task shares what is left of the task's budget, a blocking executor of another crate
    /// included. Such an executor polls again as soon as it is woken, inside the same poll of the
    /// task, where its operation yields again, so once the budget is spent it never gets its
    /// operation done. Its future has to run inside [`unconstrained`](crate::unconstrained),
    /// which lifts the budget for each poll of it. Polled there, or outside the poll of any runtime
    /// of this crate, this has no limit.
    pub fn poll_io<T>(
        &self,
        cx: &mut Context<'_>,
        interest: Interest,
        mut operation: impl FnMut() -> io::Result<T>,
    ) -> Poll<io::Result<T>> {
        ready!(coop::poll_proceed(cx));
        match operation() {
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
            result => {
                coop::spend();

                return Poll::Ready(result);
            }
        }

        // Cloned before the lock is taken: a waker's clone is somebody else's code, which is not
        // to run with a lock of this runtime's held.
        let waker = cx.waker().clone();
        let stored = {
            let mut sources = self.core.reactor.sources.lock();
            let Some(state) = sources.states.get_mut(&self.key) else {
                unreachable!("a source stays in the map for as long as its registration lives");
            };
            match state.arm(self.key, interest, &self.core.remote.poller) {
                Ok(told) => Ok((
                    told,
                    state.wakers.get_mut(interest).operation.replace(waker),
                )),
                Err(e) => Err((e, waker)),
            }
        };
        // Clear of the lock: dropping a waker can drop a task, and the future of that task may
        // hold a registration or a timer of this reactor, each of which takes a lock as it goes.
        let told = match stored {
            Ok((told, replaced)) => {
                drop(replaced);

                told
            }
            Err((e, waker)) => {
                drop(waker);

                return Poll::Ready(Err(e));
            }
        };
        // A poller that sees a change only in the wait after the one under way has that wait
        // broken here, so that the one after it watches this direction.
        if told && !Poller::LIVE {
            self.core.remote.notify();
        }

        Poll::Pending
    }

    /// Waits until the source of this registration is ready for `interest`, without running any
    /// operation on it.
    ///
    /// Use this for I/O done some other way: through a library that the source is handed to, for
    /// example, or with an operation that should run once after the wait rather than on every poll.
    ///
    /// The wait completes once the runtime finds the source ready for `interest`, or once a wait of
    /// the runtime fails. Nothing else completes it. Its first poll only stores the waker, and a
    /// poll for any other reason, such as another future in the same task, leaves it waiting.
    ///
    /// Any number of these can wait at once, on one source and in one direction, alongside an
    /// operation of [`poll_io`](Registration::poll_io). Readiness wakes them all. Dropping one
    /// gives up its wait.
    ///
    /// Readiness is only a hint: another task may take the bytes, or the room, first. A failed wait
    /// of the runtime also ends every wait for readiness, so that each waiter tries its operation
    /// and sees the result for itself. So an operation run after this completes must still expect
    /// [`WouldBlock`](io::ErrorKind::WouldBlock), and wait again when it gets one.
    ///
    /// # Errors
    ///
    /// Fails on its first poll if the OS refuses to watch the source for `interest`.
    pub fn ready(&self, interest: Interest) -> Readiness<'_, M> {
        Readiness {
            registration: self,
            interest,
            id: None,
        }
    }

    /// A handle on the runtime this registration's source is watched by.
    #[cfg(any(
        feature = "tcp",
        all(feature = "unix", unix),
        all(
            feature = "process",
            any(
                target_vendor = "apple",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd",
                target_os = "dragonfly"
            )
        )
    ))]
    pub(crate) fn runtime(&self) -> crate::Runtime<M> {
        crate::Runtime {
            core: self.core.clone(),
        }
    }
}

impl<M> Drop for Registration<M>
where
    M: Mode,
{
    fn drop(&mut self) {
        // The entry goes with this registration, and with it the reactor's clone of the source.
        let (removed, deleted) = {
            let mut sources = self.core.reactor.sources.lock();
            let removed = sources.states.remove(&self.key);
            let deleted = match &removed {
                Some(state) => {
                    sources.descriptors.remove(&state.descriptor);
                    // A poller that copied its list as the wait under way started watches the
                    // descriptor until that wait returns, and the source is kept until then, so
                    // that the descriptor stays open for it.
                    if sources.waiting {
                        sources.retired.push(state.source.clone());
                    }
                    // While the entry still holds the source, so that the poller lets go of the
                    // descriptor before it can close, and under the lock, so that no change to
                    // what the source is watched for reaches the poller after this.
                    self.core
                        .remote
                        .poller
                        .delete(self.key, state.descriptor, state.armed)
                }
                None => Ok(()),
            };

            (removed, deleted)
        };
        // Clear of the lock: the entry that came out of the map may hold a waker, and dropping a
        // waker can drop a task whose future holds a registration or a timer of this reactor,
        // each of which takes a lock as it goes. The source itself may be the last clone of
        // what was registered, whose destructor is somebody else's code.
        drop(removed);
        if let Err(e) = deleted {
            error!("The runtime failed to stop watching a source: {}", e);
        }
        // Clear of the lock as well: a wait that copied the source in is broken, so that the
        // source it keeps open goes, and a helper waiting with nothing else left to watch hears
        // that it has nothing left to wait for.
        self.core.remote.notify();
    }
}

/// A wait for a registered source to be ready for an [`Interest`], returned by
/// [`Registration::ready`].
///
/// It completes once the runtime finds the source ready, or once a wait of the runtime fails.
/// Dropping it before then gives up the wait. It fails on its first poll if the OS refuses to watch
/// the source for the interest.
#[must_use = "futures do nothing unless polled"]
pub struct Readiness<'a, M = Local>
where
    M: Mode,
{
    registration: &'a Registration<M>,
    interest: Interest,
    /// The id this wait is stored under in the reactor's map: taken on the first poll, and given
    /// up once a poll finds that the reactor took the wait out, which is what readiness does.
    id: Option<u64>,
}

impl<M> fmt::Debug for Readiness<'_, M>
where
    M: Mode,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Readiness")
            .field("interest", &self.interest)
            .finish_non_exhaustive()
    }
}

impl<M> Future for Readiness<'_, M>
where
    M: Mode,
{
    type Output = io::Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        // Cloned before the lock is taken: a waker's clone is somebody else's code, which is not
        // to run with a lock of this runtime's held.
        let waker = cx.waker().clone();
        let (done, told, unused) = {
            let mut sources = this.registration.core.reactor.sources.lock();
            let Sources {
                states,
                next_waiter,
                ..
            } = &mut *sources;
            let key = this.registration.key;
            let Some(state) = states.get_mut(&key) else {
                unreachable!("a source stays in the map for as long as its registration lives");
            };
            match this.id {
                None => {
                    match state.arm(key, this.interest, &this.registration.core.remote.poller) {
                        Ok(told) => {
                            let id = *next_waiter;
                            *next_waiter += 1;
                            let waiting = &mut state.wakers.get_mut(this.interest).readiness;
                            waiting.insert(id, waker);
                            this.id = Some(id);

                            (None, told, None)
                        }
                        // The runtime cannot watch the source for this, which the caller hears of
                        // rather than waits for readiness that nothing would report.
                        Err(e) => (Some(Err(e)), false, Some(waker)),
                    }
                }
                Some(id) => match state.wakers.get_mut(this.interest).readiness.get_mut(&id) {
                    // Still there, so the reactor has not found the source ready since: this
                    // poll came from somewhere else, and the wait goes on with its waker.
                    Some(stored) => (None, false, Some(mem::replace(stored, waker))),
                    // Taken out by the reactor, along with the waker it then woke.
                    None => (Some(Ok(())), false, Some(waker)),
                },
            }
        };
        // Clear of the lock: dropping a waker can drop a task, and the future of that task may
        // hold a registration or a timer of this reactor, each of which takes a lock as it goes.
        drop(unused);
        if let Some(outcome) = done {
            this.id = None;

            return Poll::Ready(outcome);
        }
        // A poller that sees a change only in the wait after the one under way has that wait
        // broken here, so that the one after it watches this direction. A waiter already stored
        // is watched for already.
        if told && !Poller::LIVE {
            this.registration.core.remote.notify();
        }

        Poll::Pending
    }
}

impl<M> Drop for Readiness<'_, M>
where
    M: Mode,
{
    fn drop(&mut self) {
        // Nothing in the map for a wait nobody polled, nor for one the source was found ready for.
        let Some(id) = self.id else {
            return;
        };
        let removed = {
            let mut sources = self.registration.core.reactor.sources.lock();
            let Sources { states, stale, .. } = &mut *sources;
            let key = self.registration.key;
            let Some(state) = states.get_mut(&key) else {
                unreachable!("a source stays in the map for as long as its registration lives");
            };
            // Watched still for this waiter, who may have been the last in its direction: looked
            // at again before the next wait.
            state.mark_stale(key, stale);

            state.wakers.get_mut(self.interest).readiness.remove(&id)
        };
        // Clear of the lock: dropping a waker can drop a task, and the future of that task may
        // hold a registration or a timer of this reactor, each of which takes a lock as it goes.
        // No wake-up for the wait under way, which may still watch the source for this waiter:
        // it costs that wait no more than one early return, and the source is let go of before
        // the wait after it unless somebody else waits on it by then.
        drop(removed);
    }
}

/// A timer this reactor wakes once its deadline has passed.
pub(crate) struct Sleep<M>
where
    M: Mode,
{
    core: M::Ptr<Core<M>>,
    /// When this timer comes due, and nothing where the clock has no such moment: a timer that
    /// never fires.
    deadline: Option<Instant>,
    /// Handed out on the first poll, which is when the timer joins the reactor's map.
    id: Option<u64>,
}

impl<M> Sleep<M>
where
    M: Mode,
{
    /// The runtime this timer belongs to.
    pub(crate) fn core(&self) -> &M::Ptr<Core<M>> {
        &self.core
    }

    /// When this timer comes due, and nothing where the clock has no such moment.
    pub(crate) fn deadline(&self) -> Option<Instant> {
        self.deadline
    }

    /// Whether this timer has no deadline the clock can name, and so never comes due.
    pub(crate) fn never_fires(&self) -> bool {
        self.deadline.is_none()
    }

    /// Moves this timer to `deadline`, or makes it one that never comes due where there is none.
    ///
    /// The waker stored for the old deadline moves with it, so a task already waiting on this
    /// timer is woken at the new deadline without being polled again. A reset only ever moves an
    /// entry a poll put in the map, and never stores one of its own, which is why it never needs
    /// to ask for a thread to fire it: the poll that stored the entry did, and the entry has kept
    /// the reactor busy ever since.
    pub(crate) fn reset(&mut self, deadline: Option<Instant>) {
        let old = self.deadline.zip(self.id);
        self.deadline = deadline;
        // Nothing in the map for a timer nobody ever polled, nor for one that never came due: the
        // next poll stores the new deadline as a first poll does, notification and all.
        let Some((old_deadline, id)) = old else {
            return;
        };
        let old_key = (old_deadline, id);

        let (notify, dropped) = {
            let mut timers = self.core.reactor.timers.lock();
            // The earliest deadline before the reset, which is what the wait under way, if any,
            // is bounded by.
            let bound = timers.pending.keys().next().copied();
            match (timers.pending.remove(&old_key), deadline) {
                (Some(waker), Some(new)) => {
                    let new_key = (new, id);
                    // The id is this timer's own, so the key cannot be anybody else's.
                    let replaced = timers.pending.insert(new_key, waker);
                    // A deadline no earlier than the one the wait under way ends at gets no
                    // wake-up: that wait ends at the old deadline, finds nothing due and waits
                    // again, which costs less than a write on every reset a keep-alive makes.
                    let earlier = bound.is_none_or(|bound| new_key < bound);

                    (earlier, replaced)
                }
                // A timer that is never to come due leaves the map, and a thread whose wait was
                // bounded by nothing but its deadline can stop waiting at once.
                (Some(waker), None) => (bound == Some(old_key), Some(waker)),
                // The timer has fired, and the reactor took it out, or a failed wait cleared the
                // map: its task was woken, or is being, and its next poll stores the new deadline.
                (None, _) => (false, None),
            }
        };
        // Clear of the lock: dropping a waker can drop a task whose future holds a timer of this
        // reactor, whose own drop takes this very lock.
        drop(dropped);
        if notify {
            self.core.remote.notify();
        }
    }
}

impl<M> Future for Sleep<M>
where
    M: Mode,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let this = self.get_mut();
        // A timer that never comes due registers nothing and stores no waker: there is nothing
        // that could ever wake it, which is what was asked for.
        let Some(deadline) = this.deadline else {
            return Poll::Pending;
        };
        if Instant::now() >= deadline {
            return Poll::Ready(());
        }

        // Cloned before the lock is taken: a waker's clone is somebody else's code, which is not
        // to run with a lock of this runtime's held.
        let waker = cx.waker().clone();
        let (first_deadline, replaced) = {
            let mut timers = this.core.reactor.timers.lock();
            let id = match this.id {
                Some(id) => id,
                None => {
                    let id = timers.next_id;
                    timers.next_id += 1;
                    this.id = Some(id);

                    id
                }
            };
            let key = (deadline, id);
            // A later poll of the same timer replaces the waker under the key it already has.
            let replaced = timers.pending.insert(key, waker);

            (timers.pending.keys().next() == Some(&key), replaced)
        };
        // Whether this poll put the deadline in rather than found it there, read before the
        // waker it replaced is let go of.
        let stored = replaced.is_none();
        // Clear of the lock: dropping a waker can drop a task, and the future of that task may
        // hold a timer of this reactor, whose own drop takes this very lock.
        drop(replaced);
        // A deadline behind one the map already holds changes nothing about the wait under way,
        // and neither does a poll that only puts a fresh waker under a key the map has. A poll
        // that stores a deadline the map had lost — a failed wait drops every timer it holds —
        // brings in an earliest deadline just as a first poll does.
        if stored && first_deadline {
            this.core.remote.notify();
        }

        Poll::Pending
    }
}

impl<M> Drop for Sleep<M>
where
    M: Mode,
{
    fn drop(&mut self) {
        // Nothing in the map for a timer nobody ever polled, and none either for one that never
        // comes due, which no poll of it registers.
        let (Some(deadline), Some(id)) = (self.deadline, self.id) else {
            return;
        };
        let key = (deadline, id);
        let (was_first, removed) = {
            let mut timers = self.core.reactor.timers.lock();
            let was_first = timers.pending.keys().next() == Some(&key);

            (was_first, timers.pending.remove(&key))
        };
        // Clear of the lock: dropping a waker can drop a task whose future holds a timer of
        // this reactor, whose own drop takes this very lock.
        drop(removed);
        // A thread whose wait is bounded by nothing but this deadline can stop waiting at once
        // rather than sit out a deadline nobody waits for any more.
        if was_first {
            self.core.remote.notify();
        }
    }
}

/// The sources the reactor watches, under the keys it reports them by.
struct Sources<M>
where
    M: Mode,
{
    states: HashMap<usize, SourceState<M>>,
    /// The descriptor of each source in `states`, which no other source may lend as well.
    descriptors: HashSet<RawSource>,
    next_key: usize,
    /// The id the next [`Readiness`] to store itself takes.
    next_waiter: u64,
    /// The keys of the sources that may be watched in a direction nobody waits in any more, each
    /// once: those a wait found ready, and those a waiter gave up on, to be looked at before the
    /// next wait.
    stale: Vec<usize>,
    /// Whether a wait is under way on a poller that copied its list as it started, which watches
    /// every descriptor it copied until it returns.
    waiting: bool,
    /// The sources whose registrations went while such a wait ran, kept so that their
    /// descriptors stay open until it returns.
    retired: Vec<M::SourcePtr>,
}

impl<M> Sources<M>
where
    M: Mode,
{
    /// Has `poller` stop watching each stale source in the directions nobody waits in any more.
    ///
    /// A source that fails to be told keeps being watched as it was, and is looked at again once a
    /// wait reports it; the first failure is what this returns, once every source has been looked
    /// at.
    fn reconcile(&mut self, poller: &Poller) -> io::Result<()> {
        let Self { states, stale, .. } = self;
        let mut result = Ok(());
        for key in stale.drain(..) {
            // A source let go of since has been let go of by the poller as well.
            let Some(state) = states.get_mut(&key) else {
                continue;
            };
            state.stale = false;
            let wanted = state.wakers.directions();
            if wanted == state.armed {
                continue;
            }
            match poller.modify(key, state.descriptor, state.armed, wanted) {
                Ok(()) => state.armed = wanted,
                Err(e) => {
                    if result.is_ok() {
                        result = Err(e);
                    }
                }
            }
        }

        result
    }
}

/// One watched source: the descriptor, who to wake for each direction of it, and what the poller
/// watches it for.
struct SourceState<M>
where
    M: Mode,
{
    source: M::SourcePtr,
    /// The descriptor the source lent when it was registered, which is what a wait watches.
    descriptor: RawSource,
    wakers: Wakers,
    /// The directions the poller watches the source in, as it was last told to, less any it
    /// reported it let go of by itself: every direction somebody waits in, and maybe more, until
    /// the source is next looked at.
    armed: Directions,
    /// Whether the source's key is in [`Sources::stale`].
    stale: bool,
}

impl<M> SourceState<M>
where
    M: Mode,
{
    /// Has `poller` watch this source, whose key is `key`, in `interest` as well, unless it does
    /// already: what a waiter in `interest` is stored after. Whether the poller had to be told.
    fn arm(&mut self, key: usize, interest: Interest, poller: &Poller) -> io::Result<bool> {
        let mut wanted = self.armed;
        match interest {
            Interest::Readable => wanted.readable = true,
            Interest::Writable => wanted.writable = true,
        }
        if wanted == self.armed {
            return Ok(false);
        }
        poller.modify(key, self.descriptor, self.armed, wanted)?;
        self.armed = wanted;

        Ok(true)
    }

    /// Lists this source, whose key is `key`, in `stale`, unless it is there already.
    fn mark_stale(&mut self, key: usize, stale: &mut Vec<usize>) {
        if !self.stale {
            self.stale = true;
            stale.push(key);
        }
    }
}

/// Who waits for each direction of one source.
#[derive(Default)]
struct Wakers {
    readable: Waiters,
    writable: Waiters,
}

impl Wakers {
    /// Who waits for the direction `interest` names.
    fn get_mut(&mut self, interest: Interest) -> &mut Waiters {
        match interest {
            Interest::Readable => &mut self.readable,
            Interest::Writable => &mut self.writable,
        }
    }

    /// The directions somebody waits in.
    fn directions(&self) -> Directions {
        Directions {
            readable: self.readable.any(),
            writable: self.writable.any(),
        }
    }
}

/// Who waits for one direction of one source.
#[derive(Default)]
struct Waiters {
    /// The operation [`Registration::poll_io`] keeps waiting, which the next one to wait in this
    /// direction takes the place of.
    operation: Option<Waker>,
    /// The waits for readiness alone, each under the id it took on its first poll: as many as
    /// there are [`Readiness`] futures waiting, each of which finds its own entry, or its
    /// absence, without a look at the others.
    readiness: HashMap<u64, Waker>,
}

impl Waiters {
    /// Whether anybody waits in this direction, which is what has the poller watch for it.
    fn any(&self) -> bool {
        self.operation.is_some() || !self.readiness.is_empty()
    }

    /// Takes every waker out, into `woken`: the direction is ready, or a failed wait has every
    /// waiter try again. A [`Readiness`] that finds itself gone on its next poll completes.
    fn take_into(&mut self, woken: &mut Vec<Waker>) {
        woken.extend(self.operation.take());
        woken.extend(self.readiness.drain().map(|(_, waker)| waker));
    }
}

/// The timers waiting for their deadline, keyed so that two of the same deadline stay apart.
#[derive(Default)]
struct Timers {
    pending: BTreeMap<(Instant, u64), Waker>,
    next_id: u64,
}

#[cfg(test)]
mod tests {
    use std::{
        io::Write,
        pin::pin,
        sync::{Mutex, atomic::AtomicUsize},
        task::Wake,
        thread,
    };

    #[cfg(unix)]
    use std::os::fd::OwnedFd;
    #[cfg(windows)]
    use std::os::windows::io::OwnedSocket;

    use ntest::timeout;
    use socket2::{SockRef, Socket};

    use super::*;
    use crate::{Shared, SharedRuntime};

    #[test]
    #[timeout(15000)]
    fn a_readable_source_wakes_its_waker() {
        let runtime = runtime();
        let reactor = &runtime.core.reactor;
        let (source, mut peer) = pair();
        let registration = runtime.register(source.clone()).unwrap();
        let (counter, waker) = counting_waker();
        let mut cx = Context::from_waker(&waker);
        assert!(read_one(&registration, &source, &mut cx).is_pending());

        peer.write_all(&[7]).unwrap();
        reactor.wait(Some(Duration::from_secs(1))).unwrap();

        assert_eq!(counter.count(), 1);
        assert!(matches!(
            read_one(&registration, &source, &mut cx),
            Poll::Ready(Ok(1))
        ));
    }

    #[test]
    #[timeout(15000)]
    fn a_source_written_before_registration_is_seen() {
        let runtime = runtime();
        let (source, mut peer) = pair();
        peer.write_all(&[7]).unwrap();
        let registration = runtime.register(source.clone()).unwrap();
        let (counter, waker) = counting_waker();

        let read = read_one(&registration, &source, &mut Context::from_waker(&waker));

        assert!(matches!(read, Poll::Ready(Ok(1))));
        assert_eq!(counter.count(), 0);
    }

    #[test]
    #[timeout(15000)]
    fn notify_breaks_a_wait() {
        let runtime = runtime();
        let reactor = &runtime.core.reactor;
        let breaker = {
            let runtime = runtime.clone();
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(50));
                runtime.core.remote.notify();
            })
        };

        let started = Instant::now();
        reactor.wait(None).unwrap();

        assert!(started.elapsed() < Duration::from_secs(1));
        breaker.join().unwrap();
    }

    /// A burst of wakes with no wait between them puts one wake-up in the channel, not ten.
    ///
    /// The wait that follows is unbounded, so it can only end on the wake-up the first of them
    /// wrote; that it ends at all is what says the burst was not swallowed along with the nine
    /// writes it saved.
    #[test]
    #[timeout(15000)]
    fn many_notifies_between_waits_write_once() {
        let runtime = runtime();
        let reactor = &runtime.core.reactor;
        // From a thread of its own: the thread driving a runtime is the one caller that needs
        // no telling, and this test has no such thread at all.
        let notifying = {
            let runtime = runtime.clone();
            thread::spawn(move || {
                let remote = &runtime.core.remote;
                remote.notify();
                assert!(remote.wake_pending(), "the first wake reached the channel");

                for _ in 0..9 {
                    remote.notify();
                }
                assert!(remote.wake_pending(), "the wake-up is there to be taken");
            })
        };
        notifying.join().unwrap();

        reactor.wait(None).unwrap();

        assert!(
            !runtime.core.remote.wake_pending(),
            "the wait took the wake-up out"
        );
    }

    /// A wait that comes between a `notify` raising the flag and its write to the channel finds
    /// the flag up and nothing to take out, and is no worse for it.
    ///
    /// The flag says a wake-up is on its way, not that it has arrived. A wait that took it for one
    /// to be read, and read the channel on its say-so, would find it empty and fail where it ought
    /// to have woken for nothing: the wait reads the channel only where the poll found it readable,
    /// and the read stops at the first error, an empty channel's included.
    #[test]
    #[timeout(15000)]
    fn a_wake_up_flagged_and_not_yet_written_fails_no_wait() {
        let runtime = runtime();
        let remote = &runtime.core.remote;
        remote.wake_pending.store(true, Ordering::Release);

        runtime.core.reactor.wait(Some(Duration::ZERO)).unwrap();

        assert!(!remote.wake_pending(), "the wait took the flag down");
    }

    #[test]
    #[timeout(15000)]
    fn a_wait_ends_at_the_nearest_deadline() {
        let runtime = runtime();
        let reactor = &runtime.core.reactor;
        let (counter, waker) = counting_waker();
        // Far enough ahead that the deadline lies past the wait under test even where the
        // machine stalls between that wait and the poll below.
        let mut sleep = pin!(runtime.sleep(Duration::from_millis(200)));
        let polled = sleep.as_mut().poll(&mut Context::from_waker(&waker));
        assert!(polled.is_pending());
        // That poll broke the wait a thread running the reactor would have been in, and this
        // stands in for it, so that the wait measured below is bounded by the deadline and by
        // nothing else.
        reactor.wait(Some(Duration::ZERO)).unwrap();

        let started = Instant::now();
        // The wake-up written when the deadline was stored may reach the channel after the drain
        // above. A wait that ends on it finds nothing due, and the next one is bounded by the
        // deadline alone.
        while counter.count() == 0 {
            reactor.wait(None).unwrap();
        }

        assert!(started.elapsed() < Duration::from_secs(2));
        assert_eq!(counter.count(), 1);
        assert!(reactor.is_idle());
    }

    /// A deadline further off than some platforms' waits take a timeout for, about 24 days in
    /// milliseconds, bounds a wait to what every platform's wait takes.
    #[test]
    #[timeout(15000)]
    fn a_deadline_further_off_than_a_wait_takes_bounds_it_to_what_a_wait_takes() {
        let runtime = runtime();
        let reactor = &runtime.core.reactor;
        let (_counter, waker) = counting_waker();
        let mut sleep = pin!(runtime.sleep(Duration::from_secs(30 * 24 * 60 * 60)));
        let polled = sleep.as_mut().poll(&mut Context::from_waker(&waker));
        assert!(polled.is_pending());

        assert_eq!(reactor.timeout(None), Some(poll::MAX_TIMEOUT));
        assert_eq!(
            reactor.timeout(Some(Duration::from_secs(1))),
            Some(Duration::from_secs(1))
        );
    }

    /// A wait bounded by a deadline further off than some platforms' waits take a timeout for
    /// runs, rather than failing.
    #[test]
    #[timeout(15000)]
    fn a_deadline_further_off_than_a_wait_takes_lets_the_wait_run() {
        let runtime = runtime();
        let reactor = &runtime.core.reactor;
        let (counter, waker) = counting_waker();
        let mut sleep = pin!(runtime.sleep(Duration::from_secs(30 * 24 * 60 * 60)));
        let polled = sleep.as_mut().poll(&mut Context::from_waker(&waker));
        assert!(polled.is_pending());

        // Bounded by the deadline alone, and ended at once by the wake-up the poll wrote.
        reactor.wait(None).unwrap();

        assert_eq!(counter.count(), 0);
    }

    #[test]
    #[timeout(15000)]
    fn a_dropped_sleep_leaves_no_deadline() {
        let runtime = runtime();
        let reactor = &runtime.core.reactor;
        let (counter, waker) = counting_waker();
        {
            let mut sleep = pin!(runtime.sleep(Duration::from_secs(60)));
            let polled = sleep.as_mut().poll(&mut Context::from_waker(&waker));
            assert!(polled.is_pending());
            assert!(!reactor.is_idle());
        }

        assert!(reactor.is_idle());
        assert_eq!(counter.count(), 0);
    }

    #[test]
    #[timeout(15000)]
    fn a_dropped_registration_stops_the_watch() {
        let runtime = runtime();
        let reactor = &runtime.core.reactor;
        let (source, mut peer) = pair();
        let registration = runtime.register(source.clone()).unwrap();
        let (counter, waker) = counting_waker();
        let mut cx = Context::from_waker(&waker);
        assert!(read_one(&registration, &source, &mut cx).is_pending());

        drop(registration);
        peer.write_all(&[7]).unwrap();
        reactor.wait(Some(Duration::from_millis(50))).unwrap();

        assert_eq!(counter.count(), 0);
        assert!(reactor.is_idle());
    }

    #[test]
    #[timeout(15000)]
    fn two_sleeps_with_one_deadline_both_fire() {
        let runtime = runtime();
        let reactor = &runtime.core.reactor;
        let (first, first_waker) = counting_waker();
        let (second, second_waker) = counting_waker();
        // Both made from the one deadline: `Runtime::sleep` reads the clock afresh for each
        // timer, and two deadlines nanoseconds apart would keep the pair apart on their own.
        let deadline = Instant::now() + Duration::from_millis(5);
        let mut one = pin!(sleep::<Shared>(&runtime.core, Some(deadline)));
        let mut other = pin!(sleep::<Shared>(&runtime.core, Some(deadline)));
        assert!(
            one.as_mut()
                .poll(&mut Context::from_waker(&first_waker))
                .is_pending()
        );
        assert!(
            other
                .as_mut()
                .poll(&mut Context::from_waker(&second_waker))
                .is_pending()
        );

        // Two entries under the one deadline: what tells them apart is the id beside it.
        assert_eq!(Lock::lock(&reactor.timers).pending.len(), 2);
        // A wait ends at the deadline or at the notification the first poll left behind, so it
        // takes as many as it takes for both of these to come due.
        while first.count() == 0 || second.count() == 0 {
            reactor.wait(None).unwrap();
        }

        assert_eq!(first.count(), 1);
        assert_eq!(second.count(), 1);
        assert!(reactor.is_idle());
    }

    #[test]
    #[timeout(15000)]
    fn a_failed_wait_wakes_everything() {
        let runtime = runtime();
        let reactor = &runtime.core.reactor;
        let (source, _peer) = pair();
        let registration = runtime.register(source.clone()).unwrap();
        let (reader, reader_waker) = counting_waker();
        let (timer, timer_waker) = counting_waker();
        let mut cx = Context::from_waker(&reader_waker);
        assert!(read_one(&registration, &source, &mut cx).is_pending());
        let mut sleep = pin!(runtime.sleep(Duration::from_secs(60)));
        let polled = sleep.as_mut().poll(&mut Context::from_waker(&timer_waker));
        assert!(polled.is_pending());

        reactor.wake_everything();

        assert_eq!(reader.count(), 1);
        assert_eq!(timer.count(), 1);
    }

    /// A timer for longer than the clock reaches never comes due.
    ///
    /// `Duration::MAX` is what a wait of "however long it takes" comes to, and the moment it
    /// names lies past the end of the standard clock. Such a timer registers nothing and stores
    /// no waker: nothing is ever going to fire it, which is what was asked for.
    #[test]
    #[timeout(15000)]
    fn a_sleep_beyond_the_clock_never_fires() {
        let runtime = runtime();
        let reactor = &runtime.core.reactor;
        let (counter, waker) = counting_waker();
        let mut sleep = pin!(runtime.sleep(Duration::MAX));

        let polled = sleep.as_mut().poll(&mut Context::from_waker(&waker));

        assert!(polled.is_pending());
        assert_eq!(counter.count(), 0);
        // Nothing went into the map, so nothing here keeps a thread on this reactor.
        assert!(reactor.is_idle());
    }

    /// A deadline stored again after a failed wait dropped it breaks the wait under way.
    ///
    /// A wait that fails wakes every timer and leaves the map empty, so each timer that had not
    /// come due is polled afresh and stores its deadline again. The thread driving the runtime
    /// may be in a wait that a source of its own keeps alive by then, and a deadline put back that
    /// way is news to that wait just as one stored for the first time would be.
    #[test]
    #[timeout(15000)]
    fn a_deadline_stored_again_breaks_the_wait() {
        let runtime = runtime();
        let reactor = &runtime.core.reactor;
        let (first, first_waker) = counting_waker();
        // Far enough ahead that it is still to come while the wait below is under way.
        let mut sleep = pin!(runtime.sleep(Duration::from_millis(500)));
        assert!(
            sleep
                .as_mut()
                .poll(&mut Context::from_waker(&first_waker))
                .is_pending()
        );
        // The wake-up that poll wrote, taken out of the channel so that the wait below can only
        // end on one written after it.
        reactor.wait(Some(Duration::ZERO)).unwrap();
        // What a failed wait falls back on: every timer woken, and the map left empty.
        reactor.wake_everything();
        assert_eq!(first.count(), 1);
        assert!(reactor.is_idle());

        let waiting = {
            let runtime = runtime.clone();
            thread::spawn(move || {
                let reactor = &runtime.core.reactor;
                let started = Instant::now();
                reactor.wait(None).unwrap();

                started.elapsed()
            })
        };
        // Long enough for the thread above to reach its wait, which has nothing to watch and no
        // deadline to end at.
        thread::sleep(Duration::from_millis(50));

        let (second, second_waker) = counting_waker();
        assert!(
            sleep
                .as_mut()
                .poll(&mut Context::from_waker(&second_waker))
                .is_pending()
        );

        assert!(waiting.join().unwrap() < Duration::from_secs(1));
        // And the deadline that wait learned of is the one the waits after it end at.
        while second.count() == 0 {
            reactor.wait(None).unwrap();
        }
        assert!(reactor.is_idle());
    }

    /// A reset moves a deadline the reactor holds, waker and all: the task waiting on the timer
    /// is woken at the new deadline without polling the timer again.
    #[test]
    #[timeout(15000)]
    fn a_reset_moves_the_deadline_and_keeps_the_waker() {
        let runtime = runtime();
        let reactor = &runtime.core.reactor;
        let (counter, waker) = counting_waker();
        let mut sleep = pin!(runtime.sleep(Duration::from_secs(60)));
        let polled = sleep.as_mut().poll(&mut Context::from_waker(&waker));
        assert!(polled.is_pending());
        let started = Instant::now();

        sleep.reset_after(Duration::from_millis(5));
        // A wait may end on the wake-up the poll or the reset wrote, find nothing due and wait
        // again: what ends the loop is the deadline the reset moved the waker to.
        while counter.count() == 0 {
            reactor.wait(None).unwrap();
        }

        assert!(started.elapsed() >= Duration::from_millis(5));
        assert!(started.elapsed() < Duration::from_secs(5));
        assert_eq!(counter.count(), 1);
        assert!(reactor.is_idle());
    }

    /// A reset to a deadline earlier than any the reactor holds breaks the wait under way, which
    /// was bounded by the old one.
    #[test]
    #[timeout(15000)]
    fn a_reset_to_an_earlier_deadline_breaks_the_wait() {
        let runtime = runtime();
        let reactor = &runtime.core.reactor;
        let (counter, waker) = counting_waker();
        let mut sleep = pin!(runtime.sleep(Duration::from_secs(60)));
        let polled = sleep.as_mut().poll(&mut Context::from_waker(&waker));
        assert!(polled.is_pending());
        // The wake-up that poll wrote, taken out of the channel so that the wait below can only
        // end on one written after it.
        reactor.wait(Some(Duration::ZERO)).unwrap();

        let waiting = {
            let runtime = runtime.clone();
            thread::spawn(move || {
                let reactor = &runtime.core.reactor;
                let started = Instant::now();
                reactor.wait(None).unwrap();

                started.elapsed()
            })
        };
        // Long enough for the thread above to reach its wait, bounded by nothing but the
        // deadline a minute ahead.
        thread::sleep(Duration::from_millis(50));

        sleep.reset_after(Duration::from_millis(5));

        assert!(waiting.join().unwrap() < Duration::from_secs(1));
        while counter.count() == 0 {
            reactor.wait(None).unwrap();
        }
        assert!(reactor.is_idle());
    }

    /// A reset to a later deadline writes no wake-up: the wait under way ends at the old
    /// deadline, finds nothing due and waits again, which costs less than a write on every reset
    /// a keep-alive makes.
    #[test]
    #[timeout(15000)]
    fn a_reset_to_a_later_deadline_writes_no_wake_up() {
        let runtime = runtime();
        let reactor = &runtime.core.reactor;
        let (_counter, waker) = counting_waker();
        let mut sleep = runtime.sleep(Duration::from_millis(500));
        let polled = Pin::new(&mut sleep).poll(&mut Context::from_waker(&waker));
        assert!(polled.is_pending());
        // The wake-up that poll wrote, taken out of the channel, and the flag down with it.
        reactor.wait(Some(Duration::ZERO)).unwrap();

        // From this thread, which drives nothing: the driving thread is spared every write
        // anyway, so it could not tell a reset that writes from one that does not.
        sleep.reset_after(Duration::from_secs(60));

        assert!(!runtime.core.remote.wake_pending());
        // The deadline did move, and it is the new one that the timer's drop takes out.
        let earliest = Lock::lock(&reactor.timers)
            .pending
            .keys()
            .next()
            .map(|(deadline, _)| *deadline);
        assert_eq!(earliest, sleep.deadline());
        drop(sleep);
        assert!(reactor.is_idle());
    }

    /// A reset of a timer nobody polled stores nothing and writes no wake-up: the reactor has no
    /// deadline of it to move, and the timer's first poll takes the new one.
    #[test]
    #[timeout(15000)]
    fn a_reset_of_an_unpolled_sleep_stores_nothing() {
        let runtime = runtime();
        let reactor = &runtime.core.reactor;
        let mut sleep = runtime.sleep(Duration::from_secs(60));

        sleep.reset_after(Duration::from_secs(30));

        assert!(reactor.is_idle());
        assert!(!runtime.core.remote.wake_pending());
    }

    /// A reset to a deadline further ahead than the clock can name takes the timer out of the
    /// reactor and lets its waker go: nothing is ever going to fire it. It breaks the wait that the
    /// old deadline bounded, as dropping the timer does.
    #[test]
    #[timeout(15000)]
    fn a_reset_beyond_the_clock_drops_the_waker() {
        let runtime = runtime();
        let reactor = &runtime.core.reactor;
        let (counter, waker) = counting_waker();
        let mut sleep = pin!(runtime.sleep(Duration::from_secs(60)));
        let polled = sleep.as_mut().poll(&mut Context::from_waker(&waker));
        assert!(polled.is_pending());
        drop(waker);
        // The test's own handle on the counter, and the waker the reactor keeps.
        assert_eq!(Arc::strong_count(&counter), 2);
        // The wake-up that poll wrote, taken out of the channel, and the flag down with it.
        reactor.wait(Some(Duration::ZERO)).unwrap();

        sleep.reset_after(Duration::MAX);
        // The timer bounded the wait under way, so the reset breaks it.
        assert!(runtime.core.remote.wake_pending());

        assert!(sleep.deadline().is_none());
        assert!(reactor.is_idle());
        assert_eq!(Arc::strong_count(&counter), 1);
        assert_eq!(counter.count(), 0);
    }

    /// A timer that fired can be reset and fires again, at its new deadline.
    ///
    /// The reactor let the timer go when it fired it, so the reset has nothing to move and the
    /// poll after it stores the new deadline, as a first poll does.
    #[test]
    #[timeout(15000)]
    fn a_sleep_that_fired_fires_again_once_reset() {
        let runtime = runtime();
        let reactor = &runtime.core.reactor;
        let (counter, waker) = counting_waker();
        let mut cx = Context::from_waker(&waker);
        let mut sleep = pin!(runtime.sleep(Duration::from_millis(5)));
        assert!(sleep.as_mut().poll(&mut cx).is_pending());
        while counter.count() == 0 {
            reactor.wait(None).unwrap();
        }
        assert!(sleep.as_mut().poll(&mut cx).is_ready());

        // Far enough ahead that the deadline is still to come at the poll just below.
        sleep.reset_after(Duration::from_millis(100));
        assert!(reactor.is_idle());
        assert!(sleep.as_mut().poll(&mut cx).is_pending());
        while counter.count() == 1 {
            reactor.wait(None).unwrap();
        }

        assert_eq!(counter.count(), 2);
        assert!(sleep.as_mut().poll(&mut cx).is_ready());
        assert!(reactor.is_idle());
    }

    /// A wait for readiness completes once the source is ready, and the reactor takes it out of
    /// its map as it finds that, so that the wait has nothing left to give up.
    #[test]
    #[timeout(15000)]
    fn a_readiness_wait_completes_once_the_source_is_ready() {
        let runtime = runtime();
        let reactor = &runtime.core.reactor;
        let (source, mut peer) = pair();
        let registration = runtime.register(source).unwrap();
        let (counter, waker) = counting_waker();
        let mut ready = registration.ready(Interest::Readable);
        assert!(poll_with(&mut ready, &waker).is_pending());
        assert!(waiters(&registration, Interest::Readable, Waiters::any));

        peer.write_all(&[7]).unwrap();
        reactor.wait(Some(Duration::from_secs(1))).unwrap();

        assert_eq!(counter.count(), 1);
        assert!(!waiters(&registration, Interest::Readable, Waiters::any));
        assert!(poll_with(&mut ready, &waker).is_ready());
    }

    /// A wait for readiness stays pending on a poll the source did not cause.
    ///
    /// A future that shares its task with others is polled whenever any of them wakes the task, a
    /// timer's wake included, and it must not take that for the source being ready. Neither the
    /// polls nor the waits that follow them, which find the source quiet, complete it or wake it.
    #[test]
    #[timeout(15000)]
    fn a_readiness_wait_stays_pending_on_a_poll_the_source_did_not_cause() {
        let runtime = runtime();
        let reactor = &runtime.core.reactor;
        let (source, _peer) = pair();
        let registration = runtime.register(source).unwrap();
        let (counter, waker) = counting_waker();
        let mut ready = registration.ready(Interest::Readable);

        assert!(poll_with(&mut ready, &waker).is_pending());
        assert!(poll_with(&mut ready, &waker).is_pending());
        // The wake-up the first poll wrote, taken out of the channel.
        reactor.wait(Some(Duration::ZERO)).unwrap();
        // Nothing was written to the peer, so this wait ends at its bound, or on a wake-up that
        // reached the channel late, and finds the source quiet either way.
        reactor.wait(Some(Duration::from_millis(50))).unwrap();

        assert_eq!(counter.count(), 0);
        assert!(poll_with(&mut ready, &waker).is_pending());
        assert!(waiters(&registration, Interest::Readable, Waiters::any));
    }

    /// Every wait for readiness in one direction is woken by that direction's readiness, each
    /// once, and each then completes.
    #[test]
    #[timeout(15000)]
    fn every_readiness_wait_in_one_direction_is_woken() {
        let runtime = runtime();
        let reactor = &runtime.core.reactor;
        let (source, mut peer) = pair();
        let registration = runtime.register(source).unwrap();
        let wakers = [counting_waker(), counting_waker(), counting_waker()];
        let mut waits = [
            registration.ready(Interest::Readable),
            registration.ready(Interest::Readable),
            registration.ready(Interest::Readable),
        ];
        for (wait, (_, waker)) in waits.iter_mut().zip(&wakers) {
            assert!(poll_with(wait, waker).is_pending());
        }
        let stored = waiters(&registration, Interest::Readable, |waiters| {
            waiters.readiness.len()
        });
        assert_eq!(stored, 3);

        peer.write_all(&[7]).unwrap();
        reactor.wait(Some(Duration::from_secs(1))).unwrap();

        for (wait, (counter, waker)) in waits.iter_mut().zip(&wakers) {
            assert_eq!(counter.count(), 1);
            assert!(poll_with(wait, waker).is_ready());
        }
    }

    /// A wait for readiness and an operation of `poll_io` waiting together are both woken by one
    /// readiness, and neither takes the other's place.
    #[test]
    #[timeout(15000)]
    fn a_readiness_wait_and_an_operation_are_woken_together() {
        let runtime = runtime();
        let reactor = &runtime.core.reactor;
        let (source, mut peer) = pair();
        let registration = runtime.register(source.clone()).unwrap();
        let (operation, operation_waker) = counting_waker();
        let (readiness, readiness_waker) = counting_waker();
        let mut operation_cx = Context::from_waker(&operation_waker);
        let mut ready = registration.ready(Interest::Readable);
        assert!(read_one(&registration, &source, &mut operation_cx).is_pending());
        assert!(poll_with(&mut ready, &readiness_waker).is_pending());
        // The operation waiting again, as a poll the source did not cause has it do, takes the
        // place of its own waker and of no other.
        assert!(read_one(&registration, &source, &mut operation_cx).is_pending());
        let stored = waiters(&registration, Interest::Readable, |waiters| {
            (waiters.operation.is_some(), waiters.readiness.len())
        });
        assert_eq!(stored, (true, 1));

        peer.write_all(&[7]).unwrap();
        reactor.wait(Some(Duration::from_secs(1))).unwrap();

        assert_eq!(operation.count(), 1);
        assert_eq!(readiness.count(), 1);
        assert!(poll_with(&mut ready, &readiness_waker).is_ready());
        assert!(matches!(
            read_one(&registration, &source, &mut operation_cx),
            Poll::Ready(Ok(1))
        ));
    }

    /// A dropped wait for readiness leaves nothing behind: its entry is out of the map, so the
    /// source is no longer watched on its account, and the waker it stored is let go of.
    #[test]
    #[timeout(15000)]
    fn a_dropped_readiness_wait_leaves_nothing_behind() {
        let runtime = runtime();
        let reactor = &runtime.core.reactor;
        let (source, mut peer) = pair();
        let registration = runtime.register(source).unwrap();
        let (counter, waker) = counting_waker();
        let mut ready = registration.ready(Interest::Readable);
        assert!(poll_with(&mut ready, &waker).is_pending());
        // The test's own handle on the counter, its waker, and the one the reactor keeps.
        assert_eq!(Arc::strong_count(&counter), 3);
        assert!(waiters(&registration, Interest::Readable, Waiters::any));

        drop(ready);

        assert!(!waiters(&registration, Interest::Readable, Waiters::any));
        assert_eq!(Arc::strong_count(&counter), 2);
        drop(waker);
        assert_eq!(Arc::strong_count(&counter), 1);
        // Nobody is left to wake for what the source now has to say.
        peer.write_all(&[7]).unwrap();
        reactor.wait(Some(Duration::from_millis(50))).unwrap();
        assert_eq!(counter.count(), 0);
    }

    /// Dropping one of two waits for readiness takes its own entry out and no other: the wait left
    /// is still woken, and the dropped one's waker is let go of.
    #[test]
    #[timeout(15000)]
    fn dropping_the_first_of_two_readiness_waits_leaves_the_second_waiting() {
        let runtime = runtime();
        let reactor = &runtime.core.reactor;
        let (source, mut peer) = pair();
        let registration = runtime.register(source).unwrap();
        let (first, first_waker) = counting_waker();
        let (second, second_waker) = counting_waker();
        let mut dropped = registration.ready(Interest::Readable);
        let mut kept = registration.ready(Interest::Readable);
        assert!(poll_with(&mut dropped, &first_waker).is_pending());
        assert!(poll_with(&mut kept, &second_waker).is_pending());

        drop(dropped);

        // The test's own handle on each counter and its waker, and for the second, the one the
        // reactor keeps.
        assert_eq!(Arc::strong_count(&first), 2);
        assert_eq!(Arc::strong_count(&second), 3);
        peer.write_all(&[7]).unwrap();
        reactor.wait(Some(Duration::from_secs(1))).unwrap();
        assert_eq!(first.count(), 0);
        assert_eq!(second.count(), 1);
        assert!(poll_with(&mut kept, &second_waker).is_ready());
    }

    /// A wait for readiness that was never polled stores nothing, and writes no wake-up, as there
    /// is no wait under way that it could be news to.
    #[test]
    #[timeout(15000)]
    fn an_unpolled_readiness_wait_stores_nothing() {
        let runtime = runtime();
        let (source, _peer) = pair();
        let registration = runtime.register(source).unwrap();

        let ready = registration.ready(Interest::Readable);
        assert!(!waiters(&registration, Interest::Readable, Waiters::any));
        drop(ready);

        assert!(!waiters(&registration, Interest::Readable, Waiters::any));
        assert!(!runtime.core.remote.wake_pending());
    }

    /// Only the poll that stores a wait for readiness writes a wake-up, and only where the
    /// platform's poller sees the change it makes in the wait after the one under way alone:
    /// that wait is broken, so that the next one watches the source. A later poll finds the
    /// wait stored, and so watched for already.
    #[test]
    #[timeout(15000)]
    fn only_the_first_poll_of_a_readiness_wait_writes_a_wake_up() {
        let runtime = runtime();
        let reactor = &runtime.core.reactor;
        let remote = &runtime.core.remote;
        let (source, _peer) = pair();
        let registration = runtime.register(source).unwrap();
        let (_counter, waker) = counting_waker();
        let mut ready = registration.ready(Interest::Readable);
        assert!(!remote.wake_pending());

        assert!(poll_with(&mut ready, &waker).is_pending());
        assert_eq!(remote.wake_pending(), !Poller::LIVE);
        // The wake-up taken out of the channel, and the flag down with it.
        reactor.wait(Some(Duration::ZERO)).unwrap();
        assert!(!remote.wake_pending());

        assert!(poll_with(&mut ready, &waker).is_pending());
        assert!(!remote.wake_pending());
    }

    /// A failed wait ends the waits for readiness: it wakes them with every other waiter, so that
    /// each tries its operation and sees the outcome for itself.
    #[test]
    #[timeout(15000)]
    fn a_failed_wait_ends_a_readiness_wait() {
        let runtime = runtime();
        let reactor = &runtime.core.reactor;
        let (source, _peer) = pair();
        let registration = runtime.register(source).unwrap();
        let (counter, waker) = counting_waker();
        let mut ready = registration.ready(Interest::Readable);
        assert!(poll_with(&mut ready, &waker).is_pending());

        reactor.wake_everything();

        assert_eq!(counter.count(), 1);
        assert!(!waiters(&registration, Interest::Readable, Waiters::any));
        assert!(poll_with(&mut ready, &waker).is_ready());
    }

    /// A wait for writability completes after one wait on a fresh connected pair, whose buffers
    /// have room.
    #[test]
    #[timeout(15000)]
    fn a_writable_readiness_wait_completes_on_a_fresh_pair() {
        let runtime = runtime();
        let reactor = &runtime.core.reactor;
        let (source, _peer) = pair();
        let registration = runtime.register(source).unwrap();
        let (counter, waker) = counting_waker();
        let mut ready = registration.ready(Interest::Writable);
        assert!(poll_with(&mut ready, &waker).is_pending());

        reactor.wait(Some(Duration::from_secs(1))).unwrap();

        assert_eq!(counter.count(), 1);
        assert!(poll_with(&mut ready, &waker).is_ready());
    }

    /// A wait for readiness is woken by the direction it waits for and by no other.
    ///
    /// A fresh pair is writable and not readable, so the wait for writability is woken by the
    /// first wait and the one for readability only by the second, once a byte has been written,
    /// and neither is woken again by the readiness the other found.
    #[test]
    #[timeout(15000)]
    fn a_readiness_wait_is_not_woken_by_the_other_direction() {
        let runtime = runtime();
        let reactor = &runtime.core.reactor;
        let (source, mut peer) = pair();
        let registration = runtime.register(source).unwrap();
        let (readable, readable_waker) = counting_waker();
        let (writable, writable_waker) = counting_waker();
        let mut reading = registration.ready(Interest::Readable);
        let mut writing = registration.ready(Interest::Writable);
        assert!(poll_with(&mut reading, &readable_waker).is_pending());
        assert!(poll_with(&mut writing, &writable_waker).is_pending());

        reactor.wait(Some(Duration::from_secs(1))).unwrap();

        assert_eq!(writable.count(), 1);
        assert_eq!(readable.count(), 0);
        assert!(poll_with(&mut writing, &writable_waker).is_ready());
        assert!(poll_with(&mut reading, &readable_waker).is_pending());

        peer.write_all(&[7]).unwrap();
        reactor.wait(Some(Duration::from_secs(1))).unwrap();

        assert_eq!(readable.count(), 1);
        assert_eq!(writable.count(), 1);
        assert!(poll_with(&mut reading, &readable_waker).is_ready());
    }

    /// A wait for readiness polled again before the readiness is woken through the waker of that
    /// poll, and the waker it replaced is let go of.
    #[test]
    #[timeout(15000)]
    fn a_readiness_wait_is_woken_through_the_waker_of_its_last_poll() {
        let runtime = runtime();
        let reactor = &runtime.core.reactor;
        let (source, mut peer) = pair();
        let registration = runtime.register(source).unwrap();
        let (first, first_waker) = counting_waker();
        let (second, second_waker) = counting_waker();
        let mut ready = registration.ready(Interest::Readable);
        assert!(poll_with(&mut ready, &first_waker).is_pending());
        assert!(poll_with(&mut ready, &second_waker).is_pending());
        // The first poll's waker is gone from the reactor, and the second's is there.
        drop(first_waker);
        assert_eq!(Arc::strong_count(&first), 1);
        assert_eq!(Arc::strong_count(&second), 3);

        peer.write_all(&[7]).unwrap();
        reactor.wait(Some(Duration::from_secs(1))).unwrap();

        assert_eq!(first.count(), 0);
        assert_eq!(second.count(), 1);
        assert!(poll_with(&mut ready, &second_waker).is_ready());
    }

    /// A wait for readiness polled with another waker once the readiness has come completes: it
    /// was woken through the waker it stored, and the one it is polled with now is let go of.
    #[test]
    #[timeout(15000)]
    fn a_readiness_wait_polled_with_another_waker_after_readiness_completes() {
        let runtime = runtime();
        let reactor = &runtime.core.reactor;
        let (source, mut peer) = pair();
        let registration = runtime.register(source).unwrap();
        let (first, first_waker) = counting_waker();
        let (second, second_waker) = counting_waker();
        let mut ready = registration.ready(Interest::Readable);
        assert!(poll_with(&mut ready, &first_waker).is_pending());

        peer.write_all(&[7]).unwrap();
        reactor.wait(Some(Duration::from_secs(1))).unwrap();
        assert_eq!(first.count(), 1);

        assert!(poll_with(&mut ready, &second_waker).is_ready());
        assert_eq!(second.count(), 0);
        // The test's own handle on the counter and its waker: the poll kept no clone of it.
        assert_eq!(Arc::strong_count(&second), 2);
    }

    /// A direction the poller watches for a waiter is watched still once a wait has woken that
    /// waiter, and let go of before the next wait, where nobody waits in it again by then.
    #[test]
    #[timeout(15000)]
    fn a_direction_nobody_waits_in_again_is_let_go_of_before_the_next_wait() {
        let runtime = runtime();
        let reactor = &runtime.core.reactor;
        let (source, mut peer) = pair();
        let registration = runtime.register(source.clone()).unwrap();
        let (counter, waker) = counting_waker();
        let mut cx = Context::from_waker(&waker);
        assert_eq!(armed(&registration), Directions::NONE);
        assert!(read_one(&registration, &source, &mut cx).is_pending());
        assert_eq!(armed(&registration), READABLE);

        peer.write_all(&[7]).unwrap();
        reactor.wait(Some(Duration::from_secs(1))).unwrap();
        assert_eq!(counter.count(), 1);
        assert_eq!(armed(&registration), READABLE);

        reactor.wait(Some(Duration::ZERO)).unwrap();
        assert_eq!(armed(&registration), Directions::NONE);
    }

    /// A waiter that comes back to a direction before the next wait, as a task woken to read does
    /// once it has read all there was, keeps the direction watched, and the poller is told
    /// nothing.
    #[test]
    #[timeout(15000)]
    fn a_waiter_back_before_the_next_wait_keeps_its_direction_watched() {
        let runtime = runtime();
        let reactor = &runtime.core.reactor;
        let (source, mut peer) = pair();
        let registration = runtime.register(source.clone()).unwrap();
        let (counter, waker) = counting_waker();
        let mut cx = Context::from_waker(&waker);
        assert!(read_one(&registration, &source, &mut cx).is_pending());
        peer.write_all(&[7]).unwrap();
        reactor.wait(Some(Duration::from_secs(1))).unwrap();
        assert_eq!(counter.count(), 1);

        assert!(matches!(
            read_one(&registration, &source, &mut cx),
            Poll::Ready(Ok(1))
        ));
        assert!(read_one(&registration, &source, &mut cx).is_pending());
        reactor.wait(Some(Duration::ZERO)).unwrap();

        assert_eq!(armed(&registration), READABLE);
        assert_eq!(counter.count(), 1);
    }

    /// A wait for readiness given up leaves its direction watched until the next wait, which lets
    /// it go where nobody else waits in it.
    #[test]
    #[timeout(15000)]
    fn a_dropped_readiness_wait_lets_its_direction_go_before_the_next_wait() {
        let runtime = runtime();
        let reactor = &runtime.core.reactor;
        let (source, _peer) = pair();
        let registration = runtime.register(source).unwrap();
        let (_counter, waker) = counting_waker();
        let mut first = registration.ready(Interest::Readable);
        let mut second = registration.ready(Interest::Readable);
        assert!(poll_with(&mut first, &waker).is_pending());
        assert!(poll_with(&mut second, &waker).is_pending());

        drop(first);
        reactor.wait(Some(Duration::ZERO)).unwrap();
        assert_eq!(armed(&registration), READABLE);

        drop(second);
        assert_eq!(armed(&registration), READABLE);
        reactor.wait(Some(Duration::ZERO)).unwrap();
        assert_eq!(armed(&registration), Directions::NONE);
    }

    /// A source whose peer has hung up, and that nobody waits on, keeps no wait from blocking:
    /// the poller does not report it, or reports it once, and a wait after that sits out its
    /// timeout.
    #[test]
    #[timeout(15000)]
    fn a_hung_up_source_nobody_waits_on_lets_the_wait_block() {
        let runtime = runtime();
        let reactor = &runtime.core.reactor;
        let (source, peer) = pair();
        let _registration = runtime.register(source).unwrap();
        drop(peer);
        reactor.wait(Some(Duration::ZERO)).unwrap();
        reactor.wait(Some(Duration::ZERO)).unwrap();

        let started = Instant::now();
        reactor.wait(Some(Duration::from_millis(100))).unwrap();

        assert!(started.elapsed() >= Duration::from_millis(90));
    }

    /// The source of a registration dropped while a wait is under way is kept until that wait
    /// returns, which is what a wait on a poller that copied the source's descriptor in as it
    /// started has a registration's drop do; and the wait lets go of it as it returns.
    ///
    /// The wait under way is one this test marks as such itself, so that the drop path is tested
    /// on every poller and with no race against a wait on another thread.
    #[test]
    #[timeout(15000)]
    fn a_source_let_go_of_during_a_wait_is_kept_until_the_wait_returns() {
        let runtime = runtime();
        let reactor = &runtime.core.reactor;
        let (source, _peer) = pair();
        let dropped = Arc::new(Mutex::new(false));
        let tracked = Tracked {
            source,
            dropped: dropped.clone(),
        };
        let registration = runtime.register(tracked).unwrap();
        Lock::lock(&reactor.sources).waiting = true;

        drop(registration);
        assert!(!*dropped.lock().unwrap());
        assert_eq!(Lock::lock(&reactor.sources).retired.len(), 1);

        reactor.wait(Some(Duration::ZERO)).unwrap();
        assert!(*dropped.lock().unwrap());
        assert!(Lock::lock(&reactor.sources).retired.is_empty());
    }

    /// A runtime of the test's own, whose reactor the test waits on as the thread driving it
    /// would. Shared, so that a test can hand it to a thread of its own.
    fn runtime() -> SharedRuntime {
        SharedRuntime::new().unwrap()
    }

    /// A counter and the waker that counts into it.
    fn counting_waker() -> (Arc<Counter>, Waker) {
        let counter = Arc::new(Counter::default());
        let waker = Waker::from(counter.clone());

        (counter, waker)
    }

    /// A source shareable between the registration a test makes and the reads or writes it
    /// makes directly: an owned descriptor of its own, behind an `Arc` of the test's own.
    #[cfg(unix)]
    type TestSource = Arc<OwnedFd>;
    #[cfg(windows)]
    type TestSource = Arc<OwnedSocket>;

    /// A connected pair: the source to register, and the far end to drive it from.
    fn pair() -> (TestSource, Socket) {
        let (near, far) = connected();
        near.set_nonblocking(true).unwrap();
        far.set_nonblocking(true).unwrap();

        (owned_source(near), far)
    }

    /// Wraps `socket` as a [`TestSource`]: one clone goes to [`register`], the other is
    /// kept for the reads and writes the tests make directly.
    #[cfg(unix)]
    fn owned_source(socket: Socket) -> TestSource {
        let owned: OwnedFd = socket.into();

        Arc::new(owned)
    }

    /// Wraps `socket` as a [`TestSource`]: one clone goes to [`register`], the other is
    /// kept for the reads and writes the tests make directly.
    #[cfg(windows)]
    fn owned_source(socket: Socket) -> TestSource {
        let owned: OwnedSocket = socket.into();

        Arc::new(owned)
    }

    /// Two sockets connected to one another.
    #[cfg(unix)]
    fn connected() -> (Socket, Socket) {
        socket2::Socket::pair(socket2::Domain::UNIX, socket2::Type::STREAM, None).unwrap()
    }

    /// Two sockets connected to one another.
    ///
    /// Winsock has no socket pair, so this is a loopback connection which a listener of its own
    /// accepts and then has no further use for.
    #[cfg(windows)]
    fn connected() -> (Socket, Socket) {
        use std::net::{TcpListener, TcpStream};

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let far = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let near = loop {
            let (accepted, _) = listener.accept().unwrap();
            // A loopback listener is reachable by anything else on the machine, so a connection
            // that is not the one made just above is turned away rather than taken for it.
            if accepted.peer_addr().unwrap() == far.local_addr().unwrap() {
                break accepted;
            }
        };

        // One-byte messages travel this pair, as they do the poller's wake pair, so neither
        // end holds a send back for the peer's acknowledgement of the one before it.
        near.set_nodelay(true).unwrap();
        far.set_nodelay(true).unwrap();

        (near.into(), far.into())
    }

    /// Reads one byte from `source`, the way a caller reads its socket.
    fn read_one(
        registration: &Registration<Shared>,
        source: &TestSource,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<usize>> {
        let mut byte = [std::mem::MaybeUninit::<u8>::uninit(); 1];

        registration.poll_io(cx, Interest::Readable, || {
            SockRef::from(source).recv(&mut byte)
        })
    }

    /// Polls `ready` once with `waker`, as a task whose waker that is would.
    ///
    /// The tests' sources are ones every poller watches, so the wait never fails.
    fn poll_with(ready: &mut Readiness<'_, Shared>, waker: &Waker) -> Poll<()> {
        Pin::new(ready)
            .poll(&mut Context::from_waker(waker))
            .map(|outcome| outcome.expect("the runtime watches the source"))
    }

    /// What `read` makes of who waits for `interest` of `registration`'s source.
    ///
    /// The lock of the sources is let go of as this returns, before the test polls or drops
    /// anything that takes it.
    fn waiters<T>(
        registration: &Registration<Shared>,
        interest: Interest,
        read: impl FnOnce(&Waiters) -> T,
    ) -> T {
        let mut sources = Lock::lock(&registration.core.reactor.sources);
        let Some(state) = sources.states.get_mut(&registration.key) else {
            unreachable!("a source stays in the map for as long as its registration lives");
        };

        read(state.wakers.get_mut(interest))
    }

    /// What the poller watches `registration`'s source for.
    fn armed(registration: &Registration<Shared>) -> Directions {
        let sources = Lock::lock(&registration.core.reactor.sources);
        let Some(state) = sources.states.get(&registration.key) else {
            unreachable!("a source stays in the map for as long as its registration lives");
        };

        state.armed
    }

    /// The readable direction alone.
    const READABLE: Directions = Directions {
        readable: true,
        writable: false,
    };

    /// A source that records that it has been dropped.
    struct Tracked {
        source: TestSource,
        dropped: Arc<Mutex<bool>>,
    }

    #[cfg(unix)]
    impl std::os::fd::AsFd for Tracked {
        fn as_fd(&self) -> std::os::fd::BorrowedFd<'_> {
            self.source.as_fd()
        }
    }

    #[cfg(windows)]
    impl std::os::windows::io::AsSocket for Tracked {
        fn as_socket(&self) -> std::os::windows::io::BorrowedSocket<'_> {
            self.source.as_socket()
        }
    }

    impl Drop for Tracked {
        fn drop(&mut self) {
            *self.dropped.lock().unwrap() = true;
        }
    }

    /// A waker that counts how often it has been woken.
    #[derive(Default)]
    struct Counter(AtomicUsize);

    impl Counter {
        /// How often this waker has been woken.
        fn count(&self) -> usize {
            self.0.load(Ordering::Acquire)
        }
    }

    impl Wake for Counter {
        fn wake(self: Arc<Self>) {
            self.wake_by_ref();
        }

        fn wake_by_ref(self: &Arc<Self>) {
            self.0.fetch_add(1, Ordering::AcqRel);
        }
    }
}
