//! A notification that tasks wait for, and that anyone may send.
//!
//! An [`Event`] keeps a queue of listeners, oldest first. [`Event::listen`] puts one at the back
//! and hands it out as an [`EventListener`], a future that completes once a notification reaches
//! it; [`Event::notify`] and [`Event::notify_additional`] mark the oldest listeners not notified
//! yet and wake the tasks that polled them. A listener takes its notification by completing, or
//! passes it on by being dropped with it.
//!
//! All of an event's state is one [`List`] behind one mutex, kept in a [`Shared`] beside a word
//! that tells a notification, without the lock, whether it would reach any listener. Both are
//! allocated by the first `listen` or `notify` rather than by [`Event::new`], which can then be a
//! `const fn`. The list is a slab: a vector of slots, each either holding the entry of one listener
//! or vacant, the vacant ones chained into a list that the next `listen` takes a slot from before
//! it grows the vector. A listener holds the index of its slot as its key, which is how it finds
//! its entry, and the entries are linked by index into a doubly-linked queue, which is how a
//! listener leaves the queue from wherever it is in it without a walk. The vector never shrinks: an
//! event keeps as many slots as it has had listeners alive at once, for as long as it lives.
//!
//! What the list relies on, and every change made under its lock keeps:
//!
//! * A slot is freed only by the listener whose key names it, as that listener completes or is
//!   dropped, and the listener lets go of its key as it frees it. So a key never names a slot
//!   another listener has taken since.
//! * The notified entries are a prefix of the queue. [`List::first_waiting`] names the first entry
//!   after them, or nothing where every entry is notified: a notification starts there rather than
//!   walking past the listeners notified already.
//! * [`List::notified`] is the number of notified entries, which is what the counting
//!   [`Event::notify`] counts against.
//! * Only an entry still waiting holds a waker: a notification takes it out as it marks the entry.
//! * [`Shared::notified`] says what the list said as the lock was last let go of: how many entries
//!   are notified while any entry waits, or [`NONE_WAITING`] while none does. Every holder of the
//!   lock writes it, through [`Locked`], before letting go.
//!
//! No waker is woken, cloned or dropped while the lock is held. Each of these runs somebody else's
//! code, which may come straight back to this event — to listen, to notify, to drop a listener —
//! and take the lock itself. A notification takes the wakers out and wakes them once it has let go
//! of the lock; a poll clones the waker it is to store with the lock let go of, and drops the one
//! it replaces the same way. So nothing but this module's own code runs under the lock, none of
//! which panics while the list keeps to what it relies on, and the lock is taken whether a panic
//! poisoned it or not: none can have left the list half-changed.
//!
//! A notification that the word says would reach nobody takes no lock. [`Event::notify`] and
//! [`Event::notify_additional`] first look at [`Shared::notified`], and take the lock only where it
//! says that a listener waits and that they would reach one: for `notify(n)`, that fewer than `n`
//! listeners are notified, and for `notify_additional(n)`, that `n` is not zero. Under the lock,
//! they count again, as the list may have changed since the word was written. Letting go of a lock
//! built on an event, with nobody waiting for it, thus costs a fence and two looks rather than a
//! turn with the lock of the list.
//!
//! The look must not miss a listener taken on another thread just before its caller checks the
//! condition it waits for. Whoever notifies changes the condition, then looks at the word; whoever
//! listens writes the word, then checks the condition: each side writes one location and then reads
//! the one the other side writes. Nothing short of a `SeqCst` fence between the write and the read,
//! on each side, keeps both reads from missing the other side's write, whatever orderings a caller
//! changes and checks its condition with. So `listen` makes one once it has let go of the lock,
//! with its listener counted in the word, and a notification makes one before the look that lets it
//! skip the lock. It looks once before the fence as well, and takes the lock at once where that
//! first look says it would reach a listener: the lock then orders it against every `listen` by
//! itself, as it did before the word was there, so a notification that wakes a listener pays for no
//! fence.
//!
//! One of the two fences comes before the other. If the notification's does, the caller's check
//! sees the condition changed. If `listen`'s does, the look after the notification's fence finds
//! the word `listen` wrote or a later one, and the notification does what it would have done under
//! the lock as that word was written: it skips the lock only where the listener was notified
//! already, or enough listeners were for a notification that counts them, and where it takes the
//! lock, it finds the list no earlier than as the word was written of, with the listener in it or
//! notified since.
//!
//! The fences are for callers the event knows nothing of. The locks and the channels of this crate
//! order what they check and change against the word without them, through `listen_unfenced`,
//! which writes the word with `SeqCst` and makes no fence, and `notify_unfenced` and
//! `notify_additional_unfenced`, which look at it with `SeqCst` and make none. These run on every
//! release of a lock and every send and receive of a channel, and a fence costs more than a
//! `SeqCst` access on some targets: on aarch64, a fence is a `dmb ish`, which waits for every
//! memory access before it to be done, where a `SeqCst` write and look are a plain `stlr` and
//! `ldar`.
//!
//! * The broadcast channel, the MPMC channel, the readers-writer lock and the barrier check and
//!   change what their waiters wait for under a lock of their own, which orders the two by itself.
//!   If the notifier's turn with that lock comes first, the check sees the change. If the waiter's
//!   turn does, its `listen` came before that turn, which came before the notifier's, which came
//!   before the look, so the look finds the word `listen` wrote or a later one.
//! * `lock::Mutex` checks and changes its flag with `SeqCst` operations: a compare-exchange whose
//!   failure is `SeqCst`, or a `SeqCst` `fetch_or`, to take it, and a `SeqCst` `fetch_sub` to
//!   release it, or to stop counting a waiter that held newcomers back from it. All `SeqCst`
//!   operations fall in one order, which keeps to the order each thread makes them in, and puts a
//!   read that misses a write before that write. A check that missed the change would come before
//!   it, and a look that missed the word before that word; with the change before the look and the
//!   word before the check, the four would go round in a circle, which no order can.
//! * `lock::Semaphore` checks and changes its counts of free permits and of waiters that hold
//!   newcomers back with `SeqCst` operations, as `lock::Mutex` does its flag: `SeqCst` loads, and
//!   compare-exchanges whose failure is `SeqCst`, to check them, and a `SeqCst` `fetch_add` to give
//!   a permit back, a `SeqCst` compare-exchange to add permits, or a `SeqCst` `fetch_sub` to stop
//!   counting a waiter, to change them, each followed by `notify_additional_unfenced`, the last
//!   only where a `SeqCst` look at the free permits finds some. The same circle rules out a check
//!   and a look that both miss. Where that look finds none, every permit that was free as a
//!   newcomer was held back has been taken since, and comes back through a release, which notifies,
//!   or was forgotten, and leaves nothing to wake a task for.
//!
//! One thing the lock gave that a look without it does not: a notification that took the lock came
//! before every later poll of a listener, so a listener it found notified already saw, on
//! completing, whatever the notification's caller had changed. A notification that skips the lock
//! lets go of nothing that a later poll takes, and a listener completing after it may check the
//! condition and find it as it was. A caller that takes a new listener before its last check ahead
//! of waiting again, as every caller in this crate does, sees the change on that check: the word
//! written for the new listener comes after the one the notification looked at, which orders the
//! check after the change as above, through the fences, the `SeqCst` order, or the caller's lock.

use std::{
    any::Any,
    fmt,
    future::Future,
    mem,
    ops::{Deref, DerefMut},
    panic::{self, AssertUnwindSafe},
    pin::Pin,
    sync::{
        Arc, Mutex, MutexGuard, OnceLock, PoisonError,
        atomic::{AtomicUsize, Ordering, fence},
    },
    task::{Context, Poll, Waker},
};

/// A notification that tasks can wait for.
///
/// A task that waits for a condition to change, such as a lock being released or a queue getting
/// room, calls [`Event::listen`] and awaits the [`EventListener`] it gets. Whoever changes the
/// condition notifies the event, which wakes the tasks that are listening.
///
/// An event carries no value and knows nothing about the condition. It only reports that something
/// changed.
///
/// An event needs no runtime and works under any executor. It can be notified from any thread, from
/// inside a task or from outside one. An [`EventListener`] is a plain [`Future`]. A notification
/// wakes the task that polled it last.
///
/// # Listen, then check
///
/// A notification reaches only the listeners that exist when it is sent. It is not kept for
/// listeners created later. So a task creates its listener first, then checks the condition, and
/// awaits the listener only if the check fails:
///
/// * A change made before the check is seen by the check.
/// * A change made after the check is heard by the listener.
///
/// If the task checks first and listens afterwards, a notification can arrive between the two and
/// go unheard, and the task waits forever.
///
/// # Which listeners are notified
///
/// Listeners are notified oldest first. A listener counts as notified from the moment a
/// notification reaches it until it completes or is dropped.
///
/// * [`Event::notify`] counts the listeners that are already notified. `notify(n)` notifies
///   listeners until at least `n` are notified, so calling `notify(1)` twice in a row notifies one
///   listener, not two. Use it when only one waiter can act on the change, such as a lock that only
///   one task can take.
/// * [`Event::notify_additional`] ignores that count. `notify_additional(n)` notifies `n` more
///   listeners. Use it when `n` more waiters can act on the change, such as a queue that gained `n`
///   items.
///
/// `notify(usize::MAX)` notifies every listener.
///
/// A listener that is dropped after it was notified passes its notification on. See
/// [`EventListener`].
///
/// # Wakers
///
/// The event records a notification before it wakes any task. So a waker can call back into the
/// same event from `wake`: it can notify the event, listen to it or drop one of its listeners. A
/// listener that another thread polls in the meantime already sees its notification and completes.
///
/// # Example
///
/// One thread raises a flag and a task waits for it. The example runs the task with `block_on`
/// from the `futures` crate, but any executor works:
///
/// ```
/// use std::{
///     sync::{
///         Arc,
///         atomic::{AtomicBool, Ordering},
///     },
///     thread,
/// };
///
/// use futures::executor::block_on;
/// use zruntime::Event;
///
/// /// A flag that is raised once, and the event that announces it.
/// struct Flag {
///     raised: AtomicBool,
///     event: Event,
/// }
///
/// let flag = Arc::new(Flag {
///     raised: AtomicBool::new(false),
///     event: Event::new(),
/// });
/// let raiser = thread::spawn({
///     let flag = flag.clone();
///     move || {
///         flag.raised.store(true, Ordering::Release);
///         flag.event.notify(usize::MAX);
///     }
/// });
///
/// block_on(async {
///     // Listen before the check, so a change that the check misses is heard by the listener.
///     let listener = flag.event.listen();
///     if !flag.raised.load(Ordering::Acquire) {
///         listener.await;
///     }
/// });
///
/// assert!(flag.raised.load(Ordering::Acquire));
/// raiser.join().expect("the other thread did not panic");
/// ```
pub struct Event {
    /// What the event and its listeners share, brought into being by the first `listen` or
    /// `notify`.
    shared: OnceLock<Arc<Shared>>,
}

impl Event {
    /// Creates an event with no listeners.
    ///
    /// This is a `const fn` and does not allocate, so an event can live in a `static`.
    pub const fn new() -> Self {
        Self {
            shared: OnceLock::new(),
        }
    }

    /// Creates a listener that completes when a notification reaches it.
    ///
    /// The listener is in the event's queue when this returns. Every notification sent from then on
    /// can reach it, even if it has not been polled yet.
    ///
    /// See [listen, then check](Event#listen-then-check) for why to create the listener before the
    /// last check of the condition it is for.
    pub fn listen(&self) -> EventListener {
        self.add_listener(Order::Fence)
    }

    /// Notifies the oldest listeners that are not notified yet, until at least `n` are notified.
    ///
    /// Listeners that were notified earlier count towards `n` until they complete or are dropped.
    /// So this notifies nobody if `n` listeners are notified already, and `notify(usize::MAX)`
    /// notifies every listener. It stops early if there are no more listeners to notify.
    ///
    /// Wakes the task that last polled each listener it notifies.
    ///
    /// Returns the number of listeners this call notified.
    ///
    /// # Panics
    ///
    /// If a waker panics, the tasks of the other notified listeners are still woken. The first
    /// panic is then raised from this call.
    ///
    /// # Example
    ///
    /// ```
    /// use std::{
    ///     future::Future,
    ///     pin::Pin,
    ///     task::{Context, Waker},
    /// };
    ///
    /// use zruntime::Event;
    ///
    /// let event = Event::new();
    /// let mut first = event.listen();
    /// let mut second = event.listen();
    /// // Poll the listeners by hand, with a waker that does nothing.
    /// let mut cx = Context::from_waker(Waker::noop());
    ///
    /// assert_eq!(event.notify(1), 1);
    /// // The first listener is still notified, so `notify(1)` notifies nobody.
    /// assert_eq!(event.notify(1), 0);
    ///
    /// assert!(Pin::new(&mut first).poll(&mut cx).is_ready());
    /// assert!(Pin::new(&mut second).poll(&mut cx).is_pending());
    /// // The first listener has completed and no longer counts, so this notifies the second.
    /// assert_eq!(event.notify(1), 1);
    /// assert!(Pin::new(&mut second).poll(&mut cx).is_ready());
    /// ```
    pub fn notify(&self, n: usize) -> usize {
        self.send(n, Notification::Counting, Order::Fence)
    }

    /// Notifies up to `n` more listeners, whether or not other listeners are notified already.
    ///
    /// These are the `n` oldest listeners that are not notified yet, or all of them if there are
    /// fewer. Wakes the task that last polled each listener it notifies.
    ///
    /// Returns the number of listeners this call notified.
    ///
    /// # Panics
    ///
    /// If a waker panics, the tasks of the other notified listeners are still woken. The first
    /// panic is then raised from this call.
    ///
    /// # Example
    ///
    /// ```
    /// use std::{
    ///     future::Future,
    ///     pin::Pin,
    ///     task::{Context, Waker},
    /// };
    ///
    /// use zruntime::Event;
    ///
    /// let event = Event::new();
    /// let mut first = event.listen();
    /// let mut second = event.listen();
    /// // Poll the listeners by hand, with a waker that does nothing.
    /// let mut cx = Context::from_waker(Waker::noop());
    ///
    /// assert_eq!(event.notify(1), 1);
    /// // There is one more thing to act on, so notify one more listener.
    /// assert_eq!(event.notify_additional(1), 1);
    ///
    /// assert!(Pin::new(&mut first).poll(&mut cx).is_ready());
    /// assert!(Pin::new(&mut second).poll(&mut cx).is_ready());
    /// ```
    pub fn notify_additional(&self, n: usize) -> usize {
        self.send(n, Notification::Additional, Order::Fence)
    }

    /// How many slots the queue of this event has, vacant ones included, or `None` before
    /// anything allocated the queue.
    #[cfg(test)]
    pub(crate) fn slots(&self) -> Option<usize> {
        self.shared.get().map(|shared| shared.lock().slots.len())
    }

    /// A listener to this event, as [`Event::listen`] takes one, but ordered against its caller's
    /// check of the condition by a `SeqCst` write of the word instead of a fence.
    ///
    /// For a caller in this crate that checks the condition with a `SeqCst` operation, or under a
    /// lock that whoever changes the condition takes too, as the module documentation says.
    #[cfg(any(feature = "broadcast", feature = "lock", feature = "mpmc"))]
    pub(crate) fn listen_unfenced(&self) -> EventListener {
        self.add_listener(Order::SeqCst)
    }

    /// Notifies as [`Event::notify`] does, but ordered against its caller's change to the condition
    /// by a `SeqCst` look at the word instead of a fence.
    ///
    /// For a caller in this crate that changes the condition with a `SeqCst` operation, or under a
    /// lock that whoever checks the condition takes too, as the module documentation says.
    #[cfg(any(feature = "broadcast", feature = "lock", feature = "mpmc"))]
    pub(crate) fn notify_unfenced(&self, n: usize) -> usize {
        self.send(n, Notification::Counting, Order::SeqCst)
    }

    /// Notifies as [`Event::notify_additional`] does, but ordered against its caller's change to
    /// the condition by a `SeqCst` look at the word instead of a fence.
    ///
    /// For a caller in this crate that changes the condition with a `SeqCst` operation, or under a
    /// lock that whoever checks the condition takes too, as the module documentation says.
    #[cfg(any(feature = "lock", feature = "mpmc"))]
    pub(crate) fn notify_additional_unfenced(&self, n: usize) -> usize {
        self.send(n, Notification::Additional, Order::SeqCst)
    }

    /// What this event and its listeners share, brought into being here where nothing has yet.
    ///
    /// A `notify` brings it into being as a `listen` does, even on an event nobody has listened
    /// to yet, rather than taking an event with nothing yet for one that nobody listens to. That
    /// keeps what a notification looks at without the lock to [`Shared::notified`] alone, which
    /// the fences of the module documentation keep from missing a `listen` made on another
    /// thread just before its caller's check of the condition. How the cell tells whether it is
    /// set is the standard library's own business, which no such argument could rest on.
    fn shared(&self) -> &Arc<Shared> {
        self.shared.get_or_init(Arc::default)
    }

    /// Puts a fresh listener in the queue, ordered by `order` against its caller's check of the
    /// condition, and hands it out.
    fn add_listener(&self, order: Order) -> EventListener {
        let shared = self.shared();
        // The guard writes the word with the listener counted, as `order` asks.
        let key = shared.lock_writing(order.write()).insert();
        // Between that word and the caller's check of the condition: the fence that pairs with the
        // one a notification makes before its look, where `order` has one.
        order.fence();

        EventListener {
            shared: shared.clone(),
            key: Some(key),
        }
    }

    /// Sends `notification` to `n` listeners, as [`List::notify`] counts them, ordered by `order`
    /// against its caller's change to the condition, and hands back how many it reached.
    ///
    /// The lock is taken only where [`Shared::notified`] says the notification would reach a
    /// listener, as the module documentation says. Inlined into each notification, so that its
    /// look at the word comes with no part of what taking the lock costs, which [`Shared::send`]
    /// keeps out of line.
    #[inline]
    fn send(&self, n: usize, notification: Notification, order: Order) -> usize {
        let Some(shared) = self.shared.get() else {
            return self.send_to_new(n, notification, order);
        };
        // A first look, with no fence before it, may only send the notification on to the lock,
        // which orders it against every `listen` by itself. Skipping the lock is left to a second
        // look, after the fence that pairs with the one `listen` makes before its caller's check,
        // where `order` has one, as the module documentation says.
        let mut reaches = shared.would_reach(n, notification, order.look());
        if !reaches && order.fence() {
            reaches = shared.would_reach(n, notification, order.look());
        }
        if !reaches {
            return 0;
        }

        shared.send(n, notification)
    }

    /// Sends a notification as [`Event::send`] does, on an event whose shared state nothing has
    /// brought into being yet, which this does first, as the doc of [`Event::shared`] says.
    ///
    /// Kept out of line, as only the first use of an event comes here: in line, its call would
    /// have every notification save registers on the way in, for a call that is all but never made.
    #[cold]
    #[inline(never)]
    fn send_to_new(&self, n: usize, notification: Notification, order: Order) -> usize {
        self.shared();

        self.send(n, notification, order)
    }
}

impl Default for Event {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (listeners, notified) = match self.shared.get() {
            Some(shared) => {
                let list = shared.lock();

                (list.len, list.notified)
            }
            None => (0, 0),
        };

        f.debug_struct("Event")
            .field("listeners", &listeners)
            .field("notified", &notified)
            .finish()
    }
}

/// A listener to an [`Event`]. It completes when a notification reaches it.
///
/// [`Event::listen`] creates one and puts it at the back of the event's queue. Every notification
/// sent from then on can reach it, even if it has not been polled yet.
///
/// A notification that reaches the listener wakes the task that polled it last, and the listener
/// completes on its next poll. Polling it again after it completed returns `Ready` at once.
///
/// # Dropping a listener
///
/// Dropping a listener removes it from the queue. If the listener was notified but had not
/// completed, it passes its notification on to the next listener:
///
/// * If [`Event::notify`] sent the notification, the next listener gets it only if no other
///   listener is notified at that point.
/// * If [`Event::notify_additional`] sent it, the next listener always gets it.
///
/// Passing a notification on wakes the next listener's task, so a waker that panics can make the
/// drop panic.
///
/// # Outliving the event
///
/// A listener can outlive its [`Event`]. Dropping the event notifies nobody. A listener that was
/// notified before that still completes. One that was not stays pending, unless a notified listener
/// passes its notification on to it when dropped.
pub struct EventListener {
    /// What the listener shares with its event, kept alive by the listener as much as by the
    /// event.
    shared: Arc<Shared>,
    /// The slot of this listener's entry in the list, until it has completed.
    key: Option<usize>,
}

impl Future for EventListener {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let this = self.get_mut();
        let Some(key) = this.key else {
            return Poll::Ready(());
        };

        // The waker to store, cloned with the lock let go of: a waker's clone is somebody else's
        // code, which is not to run with the lock held. Only a listener whose stored waker would
        // not wake the same task needs one, which only the list can tell, so the clone is made
        // once the list has said so, and the list looked at a second time with it in hand.
        // Nothing but a notification can have changed this listener's entry in between: nobody
        // else polls it. A clone left unused is dropped as this returns, clear of the lock too.
        let mut clone = None;
        let mut polled = this.shared.lock().poll(key, cx.waker(), &mut clone);
        if let Polled::WantsClone = polled {
            clone = Some(cx.waker().clone());
            polled = this.shared.lock().poll(key, cx.waker(), &mut clone);
        }

        match polled {
            Polled::Notified => {
                this.key = None;

                Poll::Ready(())
            }
            Polled::Waiting(replaced) => {
                // Clear of the lock: dropping a waker can drop a task, whose future may hold a
                // listener of this event.
                drop(replaced);

                Poll::Pending
            }
            Polled::WantsClone => {
                unreachable!("a second look, with a clone in hand, never asks for one")
            }
        }
    }
}

impl Drop for EventListener {
    fn drop(&mut self) {
        let Some(key) = self.key.take() else {
            return;
        };

        let mut wakers = Wakers::default();
        let removed = {
            let mut list = self.shared.lock();
            let removed = list.remove(key);
            // Passed on as it was sent: a counting notification only if no other listener is
            // notified at this point, an additional one regardless.
            if let State::Notified(notification) = removed {
                list.notify(1, notification, &mut wakers);
            }

            removed
        };
        // Clear of the lock: a wake is somebody else's code, which may come back to this event, and
        // so is the drop of the waker the entry that came out may hold, which can drop a task
        // whose future holds a listener of this event. The wakes come first, so that a drop that
        // panics cannot keep the listener passed to from being woken.
        wakers.wake();
        drop(removed);
    }
}

impl fmt::Debug for EventListener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // A listener that completed was notified before it did.
        let notified = self
            .key
            .is_none_or(|key| matches!(self.shared.lock().entry(key).state, State::Notified(_)));

        f.debug_struct("EventListener")
            .field("notified", &notified)
            .finish()
    }
}

/// What an event and its listeners share: the list of the listeners, behind a lock, and the word
/// that tells a notification, without the lock, whether it would reach any of them.
struct Shared {
    /// The listeners, taken through [`Shared::lock`] alone.
    list: Mutex<List>,
    /// How many listeners are notified, while any listener waits, or [`NONE_WAITING`] while none
    /// does, as the list said when its lock was let go of.
    ///
    /// Written under the lock alone, so the order of the writes is the order the lock was taken
    /// in, and the latest says what the list says. Read without it by a notification, and only to
    /// tell whether to take the lock: one that takes it counts again under it.
    ///
    /// Written with `Release` and read with `Acquire`, so that a notification that skips the lock
    /// still comes after the change to the list that the word it read was written of, and after
    /// whatever came before that change, as it would had it taken the lock: the drop of the last
    /// listener, say. Keeping the look from missing a `listen` is the business of the fences of
    /// the module documentation, which no ordering of the word's own accesses could do.
    notified: AtomicUsize,
}

impl Shared {
    /// Sends `notification` to `n` listeners, as [`List::notify`] counts them, under the lock, and
    /// hands back how many it reached, once it has woken the tasks they were polled by.
    ///
    /// Kept out of line: the registers it saves and the room it takes on the stack would otherwise
    /// come with every notification, the ones that skip the lock included.
    #[inline(never)]
    fn send(&self, n: usize, notification: Notification) -> usize {
        let mut wakers = Wakers::default();
        let notified = self.lock().notify(n, notification, &mut wakers);
        // Clear of the lock: a wake is somebody else's code, which may come back to this event.
        wakers.wake();

        notified
    }

    /// The list, locked, which writes [`Shared::notified`] as it lets go.
    fn lock(&self) -> Locked<'_> {
        self.lock_writing(Ordering::Release)
    }

    /// The list, locked, which writes [`Shared::notified`] with `write` as it lets go: `Release`,
    /// or `SeqCst` for a listener put in it by `listen_unfenced`.
    fn lock_writing(&self, write: Ordering) -> Locked<'_> {
        Locked {
            list: lock(&self.list),
            word: &self.notified,
            write,
        }
    }

    /// Whether `notification`, sent to `n` listeners, would reach any of them, as told by the word,
    /// loaded with `look`, without the lock.
    fn would_reach(&self, n: usize, notification: Notification, look: Ordering) -> bool {
        let notified = self.notified.load(look);

        match notification {
            // Counts the listeners notified already, against `n`. `NONE_WAITING` is past every
            // `n`, as a notification reaches none where none waits.
            Notification::Counting => notified < n,
            Notification::Additional => n > 0 && notified != NONE_WAITING,
        }
    }
}

impl Default for Shared {
    fn default() -> Self {
        Self {
            list: Mutex::default(),
            notified: AtomicUsize::new(NONE_WAITING),
        }
    }
}

/// The list of an event, locked: it dereferences to the list, and writes what the list says to
/// [`Shared::notified`] as it lets go of the lock.
struct Locked<'a> {
    list: MutexGuard<'a, List>,
    word: &'a AtomicUsize,
    /// How the word is written.
    write: Ordering,
}

impl Deref for Locked<'_> {
    type Target = List;

    fn deref(&self) -> &List {
        &self.list
    }
}

impl DerefMut for Locked<'_> {
    fn deref_mut(&mut self) -> &mut List {
        &mut self.list
    }
}

impl Drop for Locked<'_> {
    fn drop(&mut self) {
        // Written before `list` is dropped, which lets go of the lock: under the lock, as the word
        // asks.
        let notified = if self.list.notified < self.list.len {
            self.list.notified
        } else {
            NONE_WAITING
        };
        self.word.store(notified, self.write);
    }
}

/// How a `listen` or a notification is ordered against its caller's check of the condition, or
/// change to it, which the module documentation goes through.
#[derive(Clone, Copy)]
enum Order {
    /// By a `SeqCst` fence, whatever orderings the caller checks or changes the condition with:
    /// what [`Event::listen`], [`Event::notify`] and [`Event::notify_additional`] do.
    Fence,
    /// By a `SeqCst` write or look at the word: what `Event::listen_unfenced`,
    /// `Event::notify_unfenced` and `Event::notify_additional_unfenced` do, for a caller in this
    /// crate that checks and changes the condition with `SeqCst` operations, or under a lock of its
    /// own.
    #[cfg(any(feature = "broadcast", feature = "lock", feature = "mpmc"))]
    SeqCst,
}

impl Order {
    /// How `listen` writes the word with its listener counted.
    fn write(self) -> Ordering {
        match self {
            Order::Fence => Ordering::Release,
            #[cfg(any(feature = "broadcast", feature = "lock", feature = "mpmc"))]
            Order::SeqCst => Ordering::SeqCst,
        }
    }

    /// How a notification looks at the word.
    fn look(self) -> Ordering {
        match self {
            Order::Fence => Ordering::Acquire,
            #[cfg(any(feature = "broadcast", feature = "lock", feature = "mpmc"))]
            Order::SeqCst => Ordering::SeqCst,
        }
    }

    /// Makes the `SeqCst` fence, where this order has one, and tells whether it did.
    fn fence(self) -> bool {
        match self {
            Order::Fence => {
                fence(Ordering::SeqCst);

                true
            }
            #[cfg(any(feature = "broadcast", feature = "lock", feature = "mpmc"))]
            Order::SeqCst => false,
        }
    }
}

/// What [`Shared::notified`] says while no listener waits: more than any count of listeners, so
/// that a notification that counts them finds as many notified as it could ask for.
const NONE_WAITING: usize = usize::MAX;

/// The listeners of one event: their entries, in the slots of a slab, and the queue they form.
#[derive(Default)]
struct List {
    /// Every slot, each holding a listener's entry or vacant.
    slots: Vec<Slot>,
    /// The first of the vacant slots, each of which names the next.
    vacant: Option<usize>,
    /// The newest listener in the queue.
    tail: Option<usize>,
    /// The oldest listener not notified yet: the first after those that are.
    first_waiting: Option<usize>,
    /// How many listeners are in the queue.
    len: usize,
    /// How many of them are notified.
    notified: usize,
}

impl List {
    /// Puts a fresh listener at the back of the queue and hands back the key of its slot.
    fn insert(&mut self) -> usize {
        let entry = Entry {
            prev: self.tail,
            next: None,
            state: State::Waiting(None),
        };
        let key = match self.vacant {
            Some(key) => {
                let Slot::Vacant(next) = mem::replace(&mut self.slots[key], Slot::Occupied(entry))
                else {
                    unreachable!("the chain of vacant slots holds vacant slots only");
                };
                self.vacant = next;

                key
            }
            None => {
                self.slots.push(Slot::Occupied(entry));

                self.slots.len() - 1
            }
        };

        if let Some(tail) = self.tail {
            self.entry_mut(tail).next = Some(key);
        }
        self.tail = Some(key);
        // Behind every listener notified so far, so the first one waiting unless another is.
        if self.first_waiting.is_none() {
            self.first_waiting = Some(key);
        }
        self.len += 1;

        key
    }

    /// Takes a notification for the listener `key` if one reached it, which takes that listener
    /// out of the queue, and stores `waker` for it otherwise.
    ///
    /// A waker is stored as a clone, which is not made here, under the lock: where one is wanted,
    /// the clone is taken from `clone` if the caller has made it, and asked for otherwise. A
    /// waker that would wake the same task as the one stored is not stored at all.
    fn poll(&mut self, key: usize, waker: &Waker, clone: &mut Option<Waker>) -> Polled {
        let State::Waiting(stored) = &mut self.entry_mut(key).state else {
            // Taken rather than passed on: this listener is the one it was for.
            self.remove(key);

            return Polled::Notified;
        };
        if stored
            .as_ref()
            .is_some_and(|stored| stored.will_wake(waker))
        {
            return Polled::Waiting(None);
        }

        match clone.take() {
            Some(clone) => Polled::Waiting(stored.replace(clone)),
            None => Polled::WantsClone,
        }
    }

    /// Takes the listener `key` out of the queue and frees its slot, handing back the state it
    /// was in.
    fn remove(&mut self, key: usize) -> State {
        let Slot::Occupied(entry) = mem::replace(&mut self.slots[key], Slot::Vacant(self.vacant))
        else {
            unreachable!("a listener's key names a slot holding its entry");
        };
        self.vacant = Some(key);

        if let Some(prev) = entry.prev {
            self.entry_mut(prev).next = entry.next;
        }
        match entry.next {
            Some(next) => self.entry_mut(next).prev = entry.prev,
            None => self.tail = entry.prev,
        }
        // An entry behind it is waiting too, where there is one: notified entries are a prefix.
        if self.first_waiting == Some(key) {
            self.first_waiting = entry.next;
        }
        self.len -= 1;
        if let State::Notified(_) = entry.state {
            self.notified -= 1;
        }

        entry.state
    }

    /// Sends `notification` to `n` listeners and hands back how many it reached, putting the
    /// wakers it takes out of their entries in `wakers`, to be woken once the lock is let go of.
    ///
    /// A counting notification counts the listeners notified already towards `n`, and an
    /// additional one does not. Either reaches the oldest listeners not notified yet.
    fn notify(&mut self, n: usize, notification: Notification, wakers: &mut Wakers) -> usize {
        let wanted = match notification {
            Notification::Counting => n.saturating_sub(self.notified),
            Notification::Additional => n,
        };

        let mut reached = 0;
        while reached < wanted {
            let Some(key) = self.first_waiting else {
                break;
            };
            let entry = self.entry_mut(key);
            let next = entry.next;
            let State::Waiting(waker) =
                mem::replace(&mut entry.state, State::Notified(notification))
            else {
                unreachable!("every entry from the first one waiting onwards is waiting");
            };
            self.first_waiting = next;
            if let Some(waker) = waker {
                wakers.push(waker);
            }
            reached += 1;
        }
        self.notified += reached;

        reached
    }

    /// The entry of the listener `key`.
    fn entry(&self, key: usize) -> &Entry {
        match &self.slots[key] {
            Slot::Occupied(entry) => entry,
            Slot::Vacant(_) => unreachable!("a listener's key names a slot holding its entry"),
        }
    }

    /// The entry of the listener `key`, to be changed.
    fn entry_mut(&mut self, key: usize) -> &mut Entry {
        match &mut self.slots[key] {
            Slot::Occupied(entry) => entry,
            Slot::Vacant(_) => unreachable!("a listener's key names a slot holding its entry"),
        }
    }
}

/// One slot of a [`List`].
enum Slot {
    /// The slot of a listener in the queue.
    Occupied(Entry),
    /// A slot nobody holds, and the vacant slot after it, if any.
    Vacant(Option<usize>),
}

/// A listener's place in the queue, and whether a notification has reached it.
struct Entry {
    /// The listener in front of this one.
    prev: Option<usize>,
    /// The listener behind this one.
    next: Option<usize>,
    /// Whether a notification has reached the listener.
    state: State,
}

/// Whether a notification has reached a listener.
enum State {
    /// None has: the listener waits, with the waker of its latest poll if it has been polled.
    Waiting(Option<Waker>),
    /// One has, and is the listener's until it completes or is dropped.
    Notified(Notification),
}

/// The kind of notification that reached a listener, and so the kind it passes on if dropped.
#[derive(Clone, Copy)]
enum Notification {
    /// Sent by [`Event::notify`], which counts the listeners notified already.
    Counting,
    /// Sent by [`Event::notify_additional`], which does not.
    Additional,
}

/// What a poll of a listener found.
enum Polled {
    /// A notification had reached the listener, which has left the queue with it.
    Notified,
    /// None had, and the listener's waker is stored, with the one it replaced, if any, to be
    /// dropped once the lock is let go of.
    Waiting(Option<Waker>),
    /// None had, and the poll's waker is to be stored, for which a clone of it is wanted.
    WantsClone,
}

/// The wakers a notification took out of the list, to be woken once its lock is let go of.
///
/// The first is held apart from the rest, so that a notification waking one task, which is most
/// of them, allocates nothing.
#[derive(Default)]
struct Wakers {
    /// The waker taken first, if any.
    first: Option<Waker>,
    /// Those taken after it, oldest first.
    rest: Vec<Waker>,
}

impl Wakers {
    /// Keeps `waker`, to be woken after those kept before it.
    fn push(&mut self, waker: Waker) {
        match self.first {
            None => self.first = Some(waker),
            Some(_) => self.rest.push(waker),
        }
    }

    /// Wakes each waker, in the order they were kept, every one of them even where one before it
    /// panics.
    ///
    /// Each waker's entry is marked notified already, so one left unwoken would leave its task
    /// waiting for a notification that has come and gone. The first panic is raised again once
    /// every waker has been woken. The payload of any after it is disposed of: dropped with a panic
    /// of its own destructor caught, as that would otherwise escape the loop, past the wakers still
    /// to wake.
    fn wake(self) {
        let mut first_panic = None;
        for waker in self.first.into_iter().chain(self.rest) {
            if let Err(panic) = panic::catch_unwind(AssertUnwindSafe(|| waker.wake())) {
                match first_panic {
                    None => first_panic = Some(panic),
                    Some(_) => dispose(panic),
                }
            }
        }

        if let Some(panic) = first_panic {
            panic::resume_unwind(panic);
        }
    }
}

/// Drops the payload of a panic that is to go no further, with a panic of its destructor caught.
///
/// The payload is somebody else's value, and its `Drop` may panic in turn. The payload of such a
/// second panic is dropped too, the same way, and only one that panics a third time is leaked, so
/// as not to follow a chain of destructors that each panic. The scheduler has a function like this
/// one too, but this module does not use it: the runtime may be left out of the build, and an event
/// needs none of it.
fn dispose(payload: Box<dyn Any + Send>) {
    let Err(payload) = panic::catch_unwind(AssertUnwindSafe(move || drop(payload))) else {
        return;
    };
    if let Err(payload) = panic::catch_unwind(AssertUnwindSafe(move || drop(payload))) {
        mem::forget(payload);
    }
}

/// The value behind a lock, taken whether or not a panic poisoned it.
///
/// The runtime has a function like this one too, but this module does not use it: the runtime may
/// be left out of the build, and an event needs none of it.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
