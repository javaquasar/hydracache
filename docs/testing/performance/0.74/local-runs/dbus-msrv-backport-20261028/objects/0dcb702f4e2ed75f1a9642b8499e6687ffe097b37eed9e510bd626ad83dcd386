//! The two flavours a runtime comes in, and what each builds its shared state from.
//!
//! A runtime's state is reached from several places at once: every handle on it, every task
//! handle, timer and registration, and the thread driving it. What those places share it through
//! is the only thing that tells the two flavours apart. A [`Local`] runtime shares it through
//! [`Rc`] and [`RefCell`], and so never leaves the thread it was made on; a [`Shared`] one shares
//! it through [`Arc`] and [`Mutex`], and may be handed to, and driven from, any thread. Everything
//! else — the scheduler, the reactor, the loop that drives them — is written once, over [`Mode`],
//! and reaches that state through the associated types of the sealed trait behind it.
//!
//! The one other difference is who may drive a runtime. A shared runtime that came out of one of
//! the registries the `helper` feature keeps has a seat, which a helper thread takes where no
//! thread is inside `block_on` on it; the sealed trait's two hooks, for `block_on` and for work
//! just handed over, are where a shared runtime looks for that seat and a local one never does.
//!
//! Each flavour also keeps, for every thread, a record of the runtime of its flavour that thread
//! drives, which is where the free `spawn` and `spawn_local` find the runtime to put a task on.
//! The record is the sealed trait's too, so that each flavour has a thread-local of its own to
//! hold it in: a pointer to a local runtime's state is not the type of one to a shared runtime's.
//!
//! Neither flavour is `Send` or `Sync` by assertion: a `Local` runtime is neither because an `Rc`
//! is neither, and a `Shared` one is both because every piece of it is.

use std::{
    borrow::Borrow,
    cell::{RefCell, RefMut},
    future::Future,
    mem::{self, ManuallyDrop},
    ops::{Deref, DerefMut},
    pin::Pin,
    rc::{self, Rc},
    sync::{self, Arc, Mutex, MutexGuard, PoisonError},
    thread,
};

use crate::{runtime::Core, scheduler::TaskWaker};

#[cfg(any(unix, target_os = "wasi"))]
use std::os::fd::{AsFd as AsSource, BorrowedFd as BorrowedSource};
#[cfg(windows)]
use std::os::windows::io::{AsSocket as AsSource, BorrowedSocket as BorrowedSource};

/// The flavour of a [`Runtime`](crate::Runtime): [`Local`] or [`Shared`].
///
/// Only those two implement it. It cannot be implemented outside this crate.
pub trait Mode: sealed::Sealed {}

/// The flavour of a runtime that stays on the thread that created it.
///
/// A local runtime runs any `'static` future, `Send` or not. It keeps its state in [`Rc`] and
/// [`RefCell`] rather than behind atomics and locks. Its handles, and the tasks, timers and
/// registrations built on it, cannot be sent to another thread.
///
/// This is a marker type. It has no values, and is only used as a type parameter.
#[derive(Debug)]
pub enum Local {}

impl Mode for Local {}

/// The flavour of a runtime that can be used from, and run on, any thread.
///
/// A shared runtime only runs `Send` futures. It keeps its state in [`Arc`] and [`Mutex`]. Its
/// handles, and the tasks, timers and registrations built on it, are `Send` and `Sync`.
///
/// This is a marker type. It has no values, and is only used as a type parameter.
#[derive(Debug)]
pub enum Shared {}

impl Mode for Shared {}

/// A source that a runtime of flavour `M` can watch for readiness, and so can wrap in an
/// [`AsyncIo`](crate::AsyncIo).
///
/// On unix, that is anything with a file descriptor (`std::os::fd::AsFd`), such as a socket, a
/// pipe, a terminal or an eventfd. Linux and Android cannot watch a regular file, a directory or
/// `/dev/null`, and waiting on one fails there. On Windows, it is anything with a socket
/// (`std::os::windows::io::AsSocket`), the only kind of handle the runtime's `select` can watch
/// there.
///
/// A [`Local`] runtime watches such a source of any type. A [`Shared`] runtime also needs it to be
/// [`Send`] and [`Sync`], because whichever thread is running the runtime holds the source while it
/// waits.
///
/// The runtime gets the descriptor of the source once, when it starts watching it, and watches that
/// descriptor from then on. So the source must keep the same descriptor open for as long as it
/// lives, as every std type does.
///
/// It is implemented for every type that qualifies, and cannot be implemented outside this crate.
pub trait Source<M = Local>: AsSource + 'static + sealed::IntoSource<M>
where
    M: Mode,
{
}

impl<T, M> Source<M> for T
where
    M: Mode,
    T: AsSource + 'static + sealed::IntoSource<M>,
{
}

/// What [`Mode`] carries, out of reach of every crate but this one.
///
/// The trait is public in name only, so that [`Mode`] can name it as its supertrait, and sits in a
/// module nobody outside can reach, so that nobody outside can implement it: the two
/// implementations below are the only two there are.
pub(crate) mod sealed {
    use super::*;

    /// What a runtime of one flavour builds its shared state from.
    pub trait Sealed: Sized + 'static {
        /// A pointer that shares a value: [`Rc`] or [`Arc`].
        ///
        /// `Unpin` whatever it points to, as both are, so that a future holding one can be
        /// polled through a plain `&mut` whatever the runtime's flavour.
        type Ptr<T>: Clone + Deref<Target = T> + Unpin;

        /// A pointer that shares a value without keeping it alive: [`rc::Weak`] or
        /// [`sync::Weak`].
        type Weak<T>: Unpin;

        /// A lock around a value: [`RefCell`] or [`Mutex`].
        type Lock<T>: Lock<T>;

        /// A task's future once its type is erased: boxed, and `Send` where the runtime is shared.
        type BoxFuture: Future<Output = ()> + Unpin + 'static;

        /// How what a task and its handle share holds the state of the task's waker: behind an
        /// [`Arc`] of its own for a local runtime, where what they share is not `Sync` and so
        /// cannot be what the waker points at, and in place for a shared one, where it is, so
        /// that a spawn allocates no waker of its own.
        type HeldWaker: Borrow<TaskWaker> + From<TaskWaker> + 'static;

        /// A registered source once its type is erased: shared, so that a wait can hold on to it
        /// while the registration goes, and `Send + Sync` where the runtime is shared.
        type SourcePtr: Clone + 'static;

        /// Who runs a runtime of this flavour where no thread is inside `block_on` on it: nobody
        /// for a local runtime, and for a shared one, the seat a helper thread takes where the
        /// runtime came out of a registry.
        #[cfg(feature = "helper")]
        type Seat: Default;

        /// Shares `value`.
        fn new_ptr<T>(value: T) -> Self::Ptr<T>;

        /// The value `ptr` points to, taken back out of it.
        ///
        /// Called on what is left of a source's pointer once its registration is gone, so that
        /// the one other holder there can be is the reactor, which keeps the source of a
        /// registration dropped while a wait was under way until that wait returns, where the
        /// platform's poller copied the source's descriptor in as the wait started.
        fn into_inner<T>(ptr: Self::Ptr<T>) -> T;

        /// A pointer to what `ptr` points to, which does not keep it alive.
        fn downgrade<T>(ptr: &Self::Ptr<T>) -> Self::Weak<T>;

        /// What `weak` points to, if it is still alive.
        fn upgrade<T>(weak: &Self::Weak<T>) -> Option<Self::Ptr<T>>;

        /// The descriptor, or the socket on Windows, of the source `source` points to.
        ///
        /// Asked for once, as the source is registered: a wait watches the descriptor it lent
        /// then, and asks for nothing, so that it runs no code of the source's.
        ///
        /// A function rather than a bound on [`Sealed::SourcePtr`]: Windows implements
        /// `AsSocket` for an `Rc` or an `Arc` of a sized type only, and a source pointer is one
        /// of a trait object.
        fn as_source(source: &Self::SourcePtr) -> BorrowedSource<'_>;

        /// A source pointer to what `ptr` points to, sharing it with whoever holds `ptr`.
        ///
        /// What a socket of the `net` module, or a pipe or exit descriptor of the `process` module,
        /// hands its runtime to watch: the socket or pipe itself, which that module goes on doing
        /// its I/O through, or the descriptor that tells of a process's exit, which it only waits
        /// on. The bound is `Send + Sync` in either flavour, which every socket, pipe and
        /// descriptor they build on is, so that one function serves both.
        #[cfg(any(
            feature = "tcp",
            feature = "udp",
            all(feature = "unix", unix),
            all(feature = "process", unix)
        ))]
        fn source_ptr<T>(ptr: Self::Ptr<T>) -> Self::SourcePtr
        where
            T: AsSource + Send + Sync + 'static;

        /// Runs `future` to completion on the calling thread, driving `core` alongside it.
        ///
        /// A hook rather than a method of the core, so that a shared runtime which came out of a
        /// registry can be driven through the seat that runtime has.
        fn block_on<F>(core: &Self::Ptr<Core<Self>>, future: F) -> F::Output
        where
            Self: Mode,
            F: Future;

        /// Sees to it that the work just handed to `core` is run. Called after that work is in
        /// place, never before.
        fn ensure_progress(core: &Self::Ptr<Core<Self>>)
        where
            Self: Mode;

        /// Records that the calling thread drives `core`, or, with `None`, that it drives none.
        ///
        /// Written by whatever puts a thread in charge of a runtime and takes it out again, along
        /// with the marker `set_driving` keeps for wakes, which is the one place it is called
        /// from. A record kept for a runtime the thread no longer drives would send a spawn to a
        /// runtime nobody runs.
        ///
        /// Written on a thread whose locals are being destroyed as on any other: the record has
        /// no destructor, so that it is there to write to for as long as the thread runs.
        fn set_driven(core: Option<&Self::Ptr<Core<Self>>>)
        where
            Self: Mode;

        /// The runtime of this flavour the calling thread drives, if it drives one.
        ///
        /// What is spawned without a handle goes on this runtime, which is the one running the
        /// code that spawns it, on a thread whose locals are being destroyed as on any other.
        fn driven() -> Option<Self::Ptr<Core<Self>>>
        where
            Self: Mode;
    }

    /// How a source of this type is handed to a runtime of the flavour `M` to watch: what
    /// [`Source`] carries, out of reach of every crate but this one.
    ///
    /// Implemented below for every type a runtime of each flavour can watch, and by nothing
    /// else, so that the bounds a [`Source`] carries are the ones these impls name.
    pub trait IntoSource<M>: Sized
    where
        M: Mode,
    {
        /// A source pointer to what `ptr` points to, sharing it with whoever holds `ptr`.
        fn source_ptr(ptr: M::Ptr<Self>) -> M::SourcePtr;
    }

    impl<T> IntoSource<Local> for T
    where
        T: AsSource + 'static,
    {
        fn source_ptr(ptr: Rc<T>) -> Rc<dyn AsSource> {
            ptr
        }
    }

    impl<T> IntoSource<Shared> for T
    where
        T: AsSource + Send + Sync + 'static,
    {
        fn source_ptr(ptr: Arc<T>) -> Arc<dyn AsSource + Send + Sync> {
            ptr
        }
    }

    /// A lock around a value, taken with no way to fail.
    ///
    /// A [`RefCell`] fails a borrow while another is out, and the runtime's lock discipline —
    /// never hold a lock across a poll, a future's drop, a waker's wake or drop, or anything else
    /// that can come back into the runtime — is what keeps that from ever happening. A [`Mutex`]
    /// fails a lock once a panic has poisoned it, and the value behind it is taken all the same:
    /// every panic the runtime can see is contained where it happens, before it could leave a
    /// value half-changed.
    pub trait Lock<T> {
        /// What holds the lock, and derefs to the value behind it.
        type Guard<'a>: DerefMut<Target = T>
        where
            Self: 'a;

        /// A lock around `value`.
        fn new(value: T) -> Self;

        /// Takes the lock.
        fn lock(&self) -> Self::Guard<'_>;

        /// The value behind the lock, reached through a unique reference with no locking at all.
        fn get_mut(&mut self) -> &mut T;
    }

    impl Sealed for Local {
        type Ptr<T> = Rc<T>;
        type Weak<T> = rc::Weak<T>;
        type Lock<T> = RefCell<T>;
        type BoxFuture = Pin<Box<dyn Future<Output = ()>>>;
        type HeldWaker = Arc<TaskWaker>;
        type SourcePtr = Rc<dyn AsSource>;
        #[cfg(feature = "helper")]
        type Seat = ();

        fn new_ptr<T>(value: T) -> Rc<T> {
            Rc::new(value)
        }

        fn into_inner<T>(ptr: Rc<T>) -> T {
            // The reactor keeps a source past its registration only while a wait runs, and a local
            // runtime's wait runs on the thread that drives it, never across a call into user
            // code: it lets go of what it kept before it wakes anyone. So no wait is under way
            // where this is called, and the registration's end left this the only holder.
            Rc::try_unwrap(ptr).unwrap_or_else(|_| {
                unreachable!(
                    "the reactor keeps a source past its registration only while a wait runs, and \
                     the wait of a local runtime runs on this thread, never across a call into \
                     user code"
                )
            })
        }

        fn downgrade<T>(ptr: &Rc<T>) -> rc::Weak<T> {
            Rc::downgrade(ptr)
        }

        fn upgrade<T>(weak: &rc::Weak<T>) -> Option<Rc<T>> {
            weak.upgrade()
        }

        fn as_source(source: &Self::SourcePtr) -> BorrowedSource<'_> {
            borrow_source(&**source)
        }

        #[cfg(any(
            feature = "tcp",
            feature = "udp",
            all(feature = "unix", unix),
            all(feature = "process", unix)
        ))]
        fn source_ptr<T>(ptr: Rc<T>) -> Rc<dyn AsSource>
        where
            T: AsSource + Send + Sync + 'static,
        {
            ptr
        }

        fn block_on<F>(core: &Rc<Core<Self>>, future: F) -> F::Output
        where
            F: Future,
        {
            Core::<Self>::block_on(core, future)
        }

        // A local runtime is run by the thread inside `block_on` on it and by nobody else.
        fn ensure_progress(_core: &Rc<Core<Self>>) {}

        fn set_driven(core: Option<&Rc<Core<Self>>>) {
            // Held weakly, so that the record keeps no runtime alive. The thread inside `block_on`
            // holds the runtime itself for as long as the record stands.
            let core = core.map_or_else(rc::Weak::new, Rc::downgrade);
            LOCAL_DRIVEN.with(|driven| replace(driven, core));
        }

        fn driven() -> Option<Rc<Core<Self>>> {
            LOCAL_DRIVEN.with(|driven| driven.borrow().upgrade())
        }
    }

    impl Sealed for Shared {
        type Ptr<T> = Arc<T>;
        type Weak<T> = sync::Weak<T>;
        type Lock<T> = Mutex<T>;
        type BoxFuture = Pin<Box<dyn Future<Output = ()> + Send>>;
        type HeldWaker = TaskWaker;
        type SourcePtr = Arc<dyn AsSource + Send + Sync>;
        /// `None` for a runtime made by [`Runtime::new`](crate::Runtime::new), which only the
        /// threads inside `block_on` on it run.
        #[cfg(feature = "helper")]
        type Seat = Option<Mutex<crate::driver::Seat>>;

        fn new_ptr<T>(value: T) -> Arc<T> {
            Arc::new(value)
        }

        fn into_inner<T>(mut ptr: Arc<T>) -> T {
            // Once the registration is gone, the one other holder there can be is the reactor,
            // which keeps the source for a wait under way on another thread whose poller copied
            // the source's descriptor in as it started. The drop of the registration broke that
            // wait, and the reactor lets go of the source as soon as the wait returns, before it
            // runs any code of somebody else's, so this waits for that thread and no longer.
            loop {
                match Arc::try_unwrap(ptr) {
                    Ok(value) => return value,
                    Err(shared) => ptr = shared,
                }
                thread::yield_now();
            }
        }

        fn downgrade<T>(ptr: &Arc<T>) -> sync::Weak<T> {
            Arc::downgrade(ptr)
        }

        fn upgrade<T>(weak: &sync::Weak<T>) -> Option<Arc<T>> {
            weak.upgrade()
        }

        fn as_source(source: &Self::SourcePtr) -> BorrowedSource<'_> {
            borrow_source(&**source)
        }

        #[cfg(any(
            feature = "tcp",
            feature = "udp",
            all(feature = "unix", unix),
            all(feature = "process", unix)
        ))]
        fn source_ptr<T>(ptr: Arc<T>) -> Arc<dyn AsSource + Send + Sync>
        where
            T: AsSource + Send + Sync + 'static,
        {
            ptr
        }

        fn block_on<F>(core: &Arc<Core<Self>>, future: F) -> F::Output
        where
            F: Future,
        {
            #[cfg(feature = "helper")]
            if core.seat.is_some() {
                return crate::driver::block_on_seated(core, future);
            }

            Core::<Self>::block_on(core, future)
        }

        fn ensure_progress(core: &Arc<Core<Self>>) {
            core.ensure_progress();
        }

        fn set_driven(core: Option<&Arc<Core<Self>>>) {
            let core = core.map_or_else(sync::Weak::new, Arc::downgrade);
            SHARED_DRIVEN.with(|driven| replace(driven, core));
        }

        fn driven() -> Option<Arc<Core<Self>>> {
            SHARED_DRIVEN.with(|driven| driven.borrow().upgrade())
        }
    }

    thread_local! {
        /// The local runtime this thread drives, and none on a thread that drives none.
        ///
        /// Beside the marker `set_driving` writes rather than in its place: that one names the
        /// runtime by an address, which is all a wake needs, while what is built here needs a
        /// runtime to build on. Like the marker, it sits in a thread-local with no destructor, so
        /// that it can always be reached: a thread whose locals are being destroyed can reach a
        /// `block_on` from the destructor of one of them, and a spawn inside that call finds its
        /// runtime here as anywhere else. A record torn down before then would leave it none, and
        /// its panic there would be an abort. A `Weak` has a destructor, so it is kept in a
        /// `ManuallyDrop`, and [`replace`] drops each value it takes out.
        ///
        /// Which leaks nothing: the record names a runtime only while the thread drives it, and
        /// whatever puts a thread in charge of a runtime clears the record as it takes the
        /// thread out again, whether it returns or unwinds. A thread cannot end inside
        /// `block_on` or in a seat, so the record it ends with is a `Weak` to nothing, which
        /// holds no allocation to free.
        static LOCAL_DRIVEN: RefCell<ManuallyDrop<rc::Weak<Core<Local>>>> =
            const { RefCell::new(ManuallyDrop::new(rc::Weak::new())) };

        /// The shared runtime this thread drives, and none on a thread that drives none: for a
        /// thread inside `block_on` on a runtime made by `Runtime::new`, and for one in the seat
        /// of a runtime that came out of a registry. Kept as [`LOCAL_DRIVEN`] is.
        static SHARED_DRIVEN: RefCell<ManuallyDrop<sync::Weak<Core<Shared>>>> =
            const { RefCell::new(ManuallyDrop::new(sync::Weak::new())) };
    }

    /// Puts `core` on `record` in place of what was there, and drops that clear of the borrow.
    fn replace<T>(record: &RefCell<ManuallyDrop<T>>, core: T) {
        let replaced = mem::replace(&mut *record.borrow_mut(), ManuallyDrop::new(core));
        drop(ManuallyDrop::into_inner(replaced));
    }

    /// The descriptor of `source`.
    #[cfg(any(unix, target_os = "wasi"))]
    fn borrow_source<S>(source: &S) -> BorrowedSource<'_>
    where
        S: AsSource + ?Sized,
    {
        source.as_fd()
    }

    /// The socket of `source`.
    #[cfg(windows)]
    fn borrow_source<S>(source: &S) -> BorrowedSource<'_>
    where
        S: AsSource + ?Sized,
    {
        source.as_socket()
    }

    impl<T> Lock<T> for RefCell<T> {
        type Guard<'a>
            = RefMut<'a, T>
        where
            T: 'a;

        fn new(value: T) -> Self {
            RefCell::new(value)
        }

        fn lock(&self) -> RefMut<'_, T> {
            self.borrow_mut()
        }

        fn get_mut(&mut self) -> &mut T {
            RefCell::get_mut(self)
        }
    }

    impl<T> Lock<T> for Mutex<T> {
        type Guard<'a>
            = MutexGuard<'a, T>
        where
            T: 'a;

        fn new(value: T) -> Self {
            Mutex::new(value)
        }

        fn lock(&self) -> MutexGuard<'_, T> {
            Mutex::lock(self).unwrap_or_else(PoisonError::into_inner)
        }

        fn get_mut(&mut self) -> &mut T {
            Mutex::get_mut(self).unwrap_or_else(PoisonError::into_inner)
        }
    }
}
