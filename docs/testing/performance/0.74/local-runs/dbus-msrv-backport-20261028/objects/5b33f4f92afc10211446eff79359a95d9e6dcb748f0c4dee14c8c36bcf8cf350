//! A cell that is set once, to a value that tasks can wait for.
//!
//! [`OnceCell`] holds a value that is set once, by an initialiser that may be asynchronous. Every
//! task then gets a reference to it. The [module documentation](super) describes what all the
//! primitives of this module promise.

use std::{
    convert::Infallible,
    fmt,
    future::{Future, poll_fn},
    pin::{Pin, pin},
    sync::OnceLock,
    task::Poll,
};

use super::{Mutex, MutexGuard};
use crate::Event;

/// A cell that is set once, by an initialiser that may await, and whose value tasks can wait for.
///
/// A cell starts empty. The first value that reaches it stays for as long as the cell lives, and
/// every task gets a reference to that one value. It is like [`std::sync::OnceLock`], but for
/// tasks. [`get_or_init`](OnceCell::get_or_init) and [`get_or_try_init`](OnceCell::get_or_try_init)
/// take an initialiser that makes a future, which may await. [`wait`](OnceCell::wait) suspends the
/// task until another task sets the value.
///
/// The cell needs no runtime and works under any executor. It is `Send` when `T` is `Send`, and
/// `Sync` when `T` is `Send` and `Sync`. Its futures can move between threads when `T` is `Send`
/// and `Sync` and the initialiser and the future it makes can move too.
///
/// # Initialisation
///
/// A task that finds the cell empty initialises it. Tasks that find it empty meanwhile wait for the
/// initialiser to end. One initialiser runs at a time, and the others take their turns in the order
/// an async [`Mutex`] serves its waiters (see [fairness](crate::lock#fairness)).
///
/// When the initialiser finishes, the value is set and every waiting task is woken. These are the
/// tasks in [`wait`](OnceCell::wait), and the tasks waiting for their turn to initialise the cell
/// or to [`set`](OnceCell::set) it. The latter find the value set, so they run no initialiser and
/// set no value. Each completes as soon as it is polled, without waiting for the tasks ahead of it
/// in line.
///
/// If an initialiser returns an error from [`get_or_try_init`](OnceCell::get_or_try_init), panics,
/// or is given up by dropping the future that runs it, the cell stays empty. The next task in line
/// runs its own initialiser. A task that only calls [`wait`](OnceCell::wait) is not an initialiser,
/// and keeps waiting.
///
/// # Re-entrancy
///
/// The future that an initialiser makes must not call [`get_or_init`](OnceCell::get_or_init),
/// [`get_or_try_init`](OnceCell::get_or_try_init) or [`set`](OnceCell::set) on its own cell, or
/// [`wait`](OnceCell::wait) for its value. It would wait for itself and never complete, as when a
/// [`std::sync::OnceLock`] is initialised from inside its own initialiser.
///
/// # No poisoning
///
/// A panic in an initialiser does not poison the cell. The cell stays empty and the next
/// initialiser runs. See the [module documentation](crate::lock#no-poisoning).
///
/// # Example
///
/// A value is created when it is first requested, by work that awaits. Later requests get the same
/// value without repeating the work:
///
/// ```
/// use std::future;
///
/// use futures::executor::block_on;
/// use zruntime::lock::OnceCell;
///
/// let cell = OnceCell::new();
///
/// block_on(async {
///     let first = cell
///         .get_or_init(|| async {
///             // Create the value here, awaiting whatever that takes.
///             future::ready(String::from("made")).await
///         })
///         .await;
///     // The cell already has a value, so this initialiser is not run.
///     let second = cell.get_or_init(|| async { String::from("again") }).await;
///
///     assert_eq!(first, "made");
///     assert!(std::ptr::eq(first, second));
/// });
/// ```
pub struct OnceCell<T> {
    /// The value, once set. Only the holder of `initializing` sets it, so the cell is never found
    /// full by a task that has just checked it empty under that lock.
    value: OnceLock<T>,
    /// Held by the one task that initialises the cell, for as long as its initialiser runs, so
    /// that the initialisers wait for each other. It is let go of as the initialiser's future is
    /// dropped, and so by one that finishes, fails, panics or is given up alike.
    initializing: Mutex<()>,
    /// Where the tasks that wait for the value listen, those that wait for their turn to
    /// initialise the cell or to set it among them, notified once it is set. It is listened to and
    /// notified with the fences of `Event::listen` and `Event::notify`: the value is read with an
    /// `Acquire` load, which does not order it against the event the way a lock would.
    ready: Event,
}

impl<T> OnceCell<T> {
    /// Creates an empty cell.
    ///
    /// This is a `const fn`, so a cell can be a `static`.
    ///
    /// # Example
    ///
    /// ```
    /// use futures::executor::block_on;
    /// use zruntime::lock::OnceCell;
    ///
    /// static NAME: OnceCell<&str> = OnceCell::new();
    ///
    /// assert_eq!(NAME.get(), None);
    /// assert_eq!(block_on(NAME.get_or_init(|| async { "zruntime" })), &"zruntime");
    /// ```
    pub const fn new() -> Self {
        Self {
            value: OnceLock::new(),
            initializing: Mutex::new(()),
            ready: Event::new(),
        }
    }

    /// The value, if the cell has one.
    ///
    /// This never waits. It returns `None` if no value has been set, including while an initialiser
    /// is running.
    ///
    /// # Example
    ///
    /// ```
    /// use futures::executor::block_on;
    /// use zruntime::lock::OnceCell;
    ///
    /// let cell = OnceCell::new();
    /// assert_eq!(cell.get(), None);
    ///
    /// block_on(cell.set(1)).expect("nobody set a new cell");
    /// assert_eq!(cell.get(), Some(&1));
    /// ```
    pub fn get(&self) -> Option<&T> {
        self.value.get()
    }

    /// A mutable reference to the value, if the cell has one.
    ///
    /// No initialiser can be running while the cell is borrowed mutably, so this does not wait.
    ///
    /// # Example
    ///
    /// ```
    /// use zruntime::lock::OnceCell;
    ///
    /// let mut cell = OnceCell::from(1);
    /// *cell.get_mut().expect("the cell was made with a value") += 1;
    ///
    /// assert_eq!(cell.get(), Some(&2));
    /// ```
    pub fn get_mut(&mut self) -> Option<&mut T> {
        self.value.get_mut()
    }

    /// Takes the value out of the cell, leaving it empty.
    ///
    /// No initialiser can be running while the cell is borrowed mutably, so this does not wait. The
    /// cell can be set again afterwards.
    ///
    /// # Example
    ///
    /// ```
    /// use zruntime::lock::OnceCell;
    ///
    /// let mut cell = OnceCell::from(String::from("taken"));
    ///
    /// assert_eq!(cell.take().as_deref(), Some("taken"));
    /// assert_eq!(cell.take(), None);
    /// ```
    pub fn take(&mut self) -> Option<T> {
        self.value.take()
    }

    /// Consumes the cell and returns its value, if it has one.
    ///
    /// # Example
    ///
    /// ```
    /// use zruntime::lock::OnceCell;
    ///
    /// assert_eq!(OnceCell::from(1).into_inner(), Some(1));
    /// assert_eq!(OnceCell::<u8>::new().into_inner(), None);
    /// ```
    pub fn into_inner(self) -> Option<T> {
        self.value.into_inner()
    }

    /// Waits for the cell to have a value and returns a reference to it.
    ///
    /// This never initialises the cell. It waits for another task to initialise the cell or to
    /// [`set`](OnceCell::set) the value, however many initialisers fail or are given up on the way.
    /// If the cell already has a value, it completes at once.
    ///
    /// # Cancel safety
    ///
    /// Dropping the future before it completes gives up the wait. It affects no other task.
    ///
    /// # Example
    ///
    /// A thread waits for a value that this thread sets:
    ///
    /// ```
    /// use std::{sync::Arc, thread};
    ///
    /// use futures::executor::block_on;
    /// use zruntime::lock::OnceCell;
    ///
    /// let cell = Arc::new(OnceCell::new());
    /// let waiter = thread::spawn({
    ///     let cell = cell.clone();
    ///     move || *block_on(cell.wait())
    /// });
    ///
    /// block_on(cell.set(5)).expect("nobody set a new cell");
    ///
    /// assert_eq!(waiter.join().expect("the other thread did not panic"), 5);
    /// ```
    pub async fn wait(&self) -> &T {
        if let Some(value) = self.value.get() {
            return value;
        }
        loop {
            // Listen before re-checking so a value set between the check and the wait is seen.
            let listener = self.ready.listen();
            if let Some(value) = self.value.get() {
                return value;
            }
            listener.await;
        }
    }

    /// Returns the value, first setting it with `init` if the cell is empty.
    ///
    /// If the cell has a value, this returns it at once without calling `init`. Otherwise this call
    /// waits for its turn to initialise the cell, behind any initialiser that is running. If the
    /// cell has a value by then, this returns it without calling `init`. If not, this calls `init`
    /// and awaits the future it makes, and the output of that future becomes the value.
    ///
    /// # Cancel safety
    ///
    /// Dropping the future before it completes gives up the call. If it was awaiting the future of
    /// `init`, that future is dropped and the cell stays empty. See the
    /// [type documentation](OnceCell#initialisation) for what this means for the other tasks, and
    /// for what not to do inside the future of `init`.
    ///
    /// # Example
    ///
    /// ```
    /// use futures::executor::block_on;
    /// use zruntime::lock::OnceCell;
    ///
    /// let cell = OnceCell::new();
    ///
    /// assert_eq!(block_on(cell.get_or_init(|| async { 1 })), &1);
    /// // The cell has its value now, and this initialiser is not called.
    /// assert_eq!(block_on(cell.get_or_init(|| async { 2 })), &1);
    /// ```
    pub async fn get_or_init<F, Fut>(&self, init: F) -> &T
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = T>,
    {
        // The error type has no values, so the `Ok` is the only result there can be.
        let Ok(value) = self
            .get_or_try_init(|| async move { Ok::<T, Infallible>(init().await) })
            .await;

        value
    }

    /// Returns the value, first setting it with `init` if the cell is empty.
    ///
    /// This works like [`get_or_init`](OnceCell::get_or_init), except that the future that `init`
    /// makes can fail.
    ///
    /// # Errors
    ///
    /// Returns the error of the future that `init` makes. The error goes to this call alone and the
    /// cell stays empty. The next task in line runs its own initialiser instead of taking the
    /// failure as the answer.
    ///
    /// # Cancel safety
    ///
    /// Dropping the future before it completes gives up the call, as for
    /// [`get_or_init`](OnceCell::get_or_init).
    ///
    /// # Example
    ///
    /// ```
    /// use futures::executor::block_on;
    /// use zruntime::lock::OnceCell;
    ///
    /// let cell = OnceCell::<u8>::new();
    ///
    /// let failed = block_on(cell.get_or_try_init(|| async { Err("not available") }));
    /// assert_eq!(failed, Err("not available"));
    /// assert_eq!(cell.get(), None);
    ///
    /// let made = block_on(cell.get_or_try_init(|| async { Ok::<_, &str>(1) }));
    /// assert_eq!(made, Ok(&1));
    /// ```
    pub async fn get_or_try_init<F, Fut, E>(&self, init: F) -> Result<&T, E>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, E>>,
    {
        if let Some(value) = self.value.get() {
            return Ok(value);
        }
        // Held until this call ends, whichever way it does: the guard is dropped with the future
        // that holds it, which lets the next initialiser in, whether this one set the value,
        // failed, panicked or was given up.
        let _initializing = match self.lock_or_value().await {
            Ok(guard) => guard,
            Err(value) => return Ok(value),
        };
        // The turn may have come together with the value, as the initialiser that set it let go of
        // the turn, which the wait prefers: look again before running an initialiser.
        if let Some(value) = self.value.get() {
            return Ok(value);
        }
        let value = init().await?;

        Ok(self.store(value))
    }

    /// Sets the value of the cell if it has none, waiting for an initialiser that is running.
    ///
    /// On success, returns a reference to the value that was set. A running initialiser gets its
    /// turn first, which is why this waits. If that initialiser sets a value, this call fails. If
    /// it fails or is given up, this call sets its value, unless another initialiser gets in first
    /// and sets one, which makes this call fail.
    ///
    /// # Errors
    ///
    /// Returns `value` in the `Err` if the cell already has a value.
    ///
    /// # Cancel safety
    ///
    /// Dropping the future before it completes gives up the call. `value` is dropped with it, and
    /// the cell is left as it was.
    ///
    /// # Example
    ///
    /// ```
    /// use futures::executor::block_on;
    /// use zruntime::lock::OnceCell;
    ///
    /// let cell = OnceCell::new();
    ///
    /// assert_eq!(block_on(cell.set(1)), Ok(&1));
    /// // The cell has its value, which stays: this one comes back.
    /// assert_eq!(block_on(cell.set(2)), Err(2));
    /// assert_eq!(cell.get(), Some(&1));
    /// ```
    pub async fn set(&self, value: T) -> Result<&T, T> {
        if self.value.get().is_some() {
            return Err(value);
        }
        let Ok(_initializing) = self.lock_or_value().await else {
            return Err(value);
        };
        // The turn may have come together with the value, as the initialiser that set it let go of
        // the turn, which the wait prefers: look again before setting a value.
        if self.value.get().is_some() {
            return Err(value);
        }

        Ok(self.store(value))
    }

    /// Waits for this call's turn to initialise the cell, which is the lock on `initializing`, or
    /// for the cell to have a value, whichever comes first, and hands out the guard of the lock or
    /// the value.
    ///
    /// The value wakes every task that waits for it, those that wait for their turn among them:
    /// the lock alone would let them out one at a time, each behind the one before it, and one
    /// that was woken and is never polled would hold back all of those behind it. Where the turn
    /// and the value come together the turn wins, and the caller looks at the value again.
    async fn lock_or_value(&self) -> Result<MutexGuard<'_, ()>, &T> {
        // Polled until it completes, and never after: the loop below ends as soon as it does.
        let mut lock = pin!(self.initializing.lock());
        loop {
            // Listen before checking so a value set in between is heard of.
            let mut set = self.ready.listen();
            if let Some(value) = self.value.get() {
                return Err(value);
            }
            let guard = poll_fn(|cx| {
                if let Poll::Ready(guard) = lock.as_mut().poll(cx) {
                    return Poll::Ready(Some(guard));
                }

                Pin::new(&mut set).poll(cx).map(|()| None)
            })
            .await;
            if let Some(guard) = guard {
                // Dropped before the guard is returned: a drop of the listener that panics then
                // unwinds with the guard still a local, which lets the lock go, where a returned
                // guard would be leaked, with the lock held for good.
                drop(set);

                return Ok(guard);
            }
        }
    }

    /// Puts `value` in the cell and wakes the tasks that wait for it, handing back a reference to
    /// it.
    ///
    /// For the holder of `initializing`, which has seen the cell empty: nobody else sets the value,
    /// so this one is the one that stays.
    fn store(&self, value: T) -> &T {
        let stored = self.value.get_or_init(|| value);
        self.ready.notify(usize::MAX);

        stored
    }
}

impl<T> Default for OnceCell<T> {
    /// Creates an empty cell.
    fn default() -> Self {
        Self::new()
    }
}

impl<T> From<T> for OnceCell<T> {
    /// Creates a cell that already holds `value`.
    fn from(value: T) -> Self {
        Self {
            value: OnceLock::from(value),
            initializing: Mutex::new(()),
            ready: Event::new(),
        }
    }
}

impl<T> fmt::Debug for OnceCell<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = f.debug_tuple("OnceCell");
        match self.value.get() {
            Some(value) => s.field(value),
            None => s.field(&format_args!("<uninit>")),
        };

        s.finish()
    }
}
