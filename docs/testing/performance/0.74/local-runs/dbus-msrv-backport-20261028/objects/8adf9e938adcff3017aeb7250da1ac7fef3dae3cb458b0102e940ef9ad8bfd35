//! Who runs a runtime with a seat: the thread inside a [`block_on`](crate::block_on), or a
//! helper thread where no such thread is there to do it.
//!
//! A shared runtime handed out by [`SharedRuntime::current`](crate::SharedRuntime::current), or
//! built on by the free `block_on`, comes out of one of the registries below, and has a seat. The
//! scheduler and the reactor are run by one thread at a time, the one in the driver's seat.
//! A thread that enters `block_on` takes the seat if it is free and keeps it until its future is
//! done: between two polls of that future it runs a batch of ready tasks and then waits on the
//! reactor, so a program that drives its work through `block_on` runs it on its own thread and
//! starts none. A thread that finds the seat taken parks instead, to be polled again when its
//! future is woken or the seat is freed.
//!
//! Work that outlives every `block_on` — a task, a registered source or a timer polled from some
//! other executor, or a task left running once `block_on` has returned — is run by a helper
//! thread, started where that work is found with nobody in the seat, and gone once nothing is
//! left to run, watch or time. A `block_on` that arrives while the helper is in the seat is given
//! it, the helper parking until that call leaves, so that a program calling `block_on` once per
//! operation runs each of them on its own thread rather than behind a thread of the runtime's
//! own.
//!
//! On WASI, no helper is ever started, as nothing in the crate starts a thread there: WASI without
//! threads, `wasm32-wasip1` and `wasm32-wasip2`, cannot start one, and no `cfg` tells a WASI target
//! with threads, such as `wasm32-wasip1-threads`, from one without, so such a target is treated the
//! same way. The seat is left free where a helper would take it, and the work waits for the next
//! `block_on` on its runtime: a runtime with a seat runs, as one made by `Runtime::new` does, only
//! while a thread is inside `block_on` on it. Nor does anything hold it in between, as a
//! helper holds the runtime it runs: one that no handle holds goes, with the work on it, as soon as
//! no `block_on` drives it. With the helper never up, what wakes or waits for it finds none, and
//! the rest of the seat works as it does anywhere: on a single thread, a `block_on` finds the seat
//! free or panics where its thread is in it already, and on a target with threads, one finds it
//! taken only by another thread's `block_on`, and parks until that call frees it.
//!
//! Which runtime a thread is in the seat of is written down in the one place every runtime keeps
//! that record, the marker and the record of the runtime itself that [`set_driving`] writes, so
//! that a wake on that thread spares itself the write that would break a wait the thread is not
//! in, and a `block_on` there is turned away.

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    future::Future,
    io,
    mem::{self, ManuallyDrop},
    pin::pin,
    sync::{
        Arc, Mutex, MutexGuard, Weak,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll, Wake, Waker},
    thread::{self, Thread, ThreadId},
};

use crate::{
    Shared, coop,
    mode::sealed::Sealed as _,
    runtime::{Core, drives, drives_a_runtime, lock, set_driving},
};

/// The runtime for what the calling thread builds, made here if none is alive: the one it is in
/// the seat of, where it is in one; the one the innermost `block_on` it is inside drives, where
/// it is inside one and so has a thread to run what it builds; and the process's otherwise.
///
/// The innermost call is the one that decides, because it is the one whose loop this thread is
/// in: a call of the free `block_on` drives the thread's own runtime, and a call on a runtime
/// handed out by `SharedRuntime::current` drives that runtime, which need not be the thread's
/// own. Work built for the thread's own runtime inside the latter would wait for a call that is
/// not coming.
///
/// Once this thread's `OWN` local is gone, there is no registry left to hold what it builds:
/// inside a call of the free `block_on`, that goes on a runtime in no registry, which a helper
/// thread runs; outside one, it still goes on the process's shared registry as before.
pub(crate) fn current() -> io::Result<Arc<Core<Shared>>> {
    if let Some(core) = driven() {
        return Ok(core);
    }
    if !in_block_on() {
        return shared_in(&SHARED);
    }
    if let Target::Core(core) = innermost() {
        return Ok(core);
    }
    match OWN.try_with(shared_in) {
        Ok(core) => core,
        Err(_) => Core::with_seat(),
    }
}

/// The runtime the free `spawn` falls back on where the calling thread drives none: [`current`],
/// and held by the `block_on` the thread is inside, if it is the thread's own runtime.
///
/// A `block_on` looks its own runtime up afresh at each turn of its loop, and takes the seat of
/// what it finds. The only hold on a runtime this call brings into being is the handle it hands
/// back, which a `spawn` lets go of as soon as it has queued its task, and the runtime would go
/// with it before the `block_on` had a turn to take its seat: the task would be dropped with it,
/// and what awaited it told that its runtime is gone. A caller of `SharedRuntime::current` keeps
/// the handle it is given, which a spawn with no handle of its own cannot. The innermost
/// `block_on` holds the runtime from here on instead, until it returns (see [`HELD`]).
pub(crate) fn current_to_spawn_on() -> io::Result<Arc<Core<Shared>>> {
    let core = current()?;
    if in_block_on() && is_own(&core) {
        HELD.with(|held| {
            let mut held = held.borrow_mut();
            if held.is_none() {
                **held = Some(core.clone());
            }
        });
    }

    Ok(core)
}

/// The calling thread's own runtime, if one is alive: what the free `block_on` resolves to.
///
/// Nothing, too, once this thread's `OWN` local is gone: a `block_on` from the destructor of
/// another local then takes no seat, and polls and parks.
pub(crate) fn own() -> Option<Arc<Core<Shared>>> {
    OWN.try_with(|own| lock(own).upgrade()).ok().flatten()
}

/// Runs `future` to completion on the calling thread, running `core`, a runtime with a seat,
/// alongside it whenever the seat is free.
pub(crate) fn block_on_seated<F>(core: &Arc<Core<Shared>>, future: F) -> F::Output
where
    F: Future,
{
    let target = Target::Core(core.clone());
    let core = core.clone();

    block_on(target, &move || Some(core.clone()), future)
}

/// What a `block_on` drives: the calling thread's own runtime, or a given one.
pub(crate) enum Target {
    /// The runtime the thread's own registry names, which the free `block_on` drives.
    Own,
    /// A runtime with a seat, which a `block_on` on that very runtime drives.
    Core(Arc<Core<Shared>>),
}

/// The runtime the calling thread is in the seat of, if it is in one.
///
/// The record this reads is written for every shared runtime a thread drives, a `block_on` on a
/// runtime with no seat among them. Such a runtime is left out here, as it always was: the thread
/// inside it is in no seat, and what [`current`] builds there goes to the runtime the process
/// shares.
///
/// The record has no destructor, so this finds the seat on a thread whose locals are being
/// destroyed too, where a `block_on` run from the destructor of one of them has taken it. What
/// [`current`] hands out there is the runtime of that seat, as on any other thread: one the thread
/// holds for as long as it is in the seat, and whose work it runs on the next turn of its loop.
/// Were it to look past the seat instead, the list of calls and the registry may be gone by then,
/// and give it another runtime than the one the thread runs, or a fresh one. Of the callers of
/// [`current`], `SharedRuntime::current` hands out there what it says it does for a thread in a
/// seat, and [`current_to_spawn_on`] is reached only where the thread drives no shared runtime at
/// all, so never from a seat.
fn driven() -> Option<Arc<Core<Shared>>> {
    Shared::driven().filter(|core| core.seat.is_some())
}

/// Whether the calling thread is inside a `block_on` of this layer, and so has a thread of its
/// own for what it builds and for the work it hands over.
///
/// `false` on a thread whose locals are being destroyed, which is a thread outside `block_on`
/// unless a `block_on` is what that destruction is running, and the count says so where it is.
pub(crate) fn in_block_on() -> bool {
    IN_BLOCK_ON
        .try_with(Cell::get)
        .is_ok_and(|inside| inside > 0)
}

/// What the innermost `block_on` the calling thread is inside drives. For a thread inside one.
///
/// [`Target::Own`] where the record of what each call drives is gone, on a thread whose locals are
/// being destroyed: what a call made there drives is looked up in the thread's own registry, as
/// it was before calls kept that record.
fn innermost() -> Target {
    let top = TARGETS.try_with(|targets| match targets.borrow().last() {
        Some(Target::Core(core)) => Some(core.clone()),
        Some(Target::Own) | None => None,
    });

    match top {
        Ok(Some(core)) => Target::Core(core),
        Ok(None) | Err(_) => Target::Own,
    }
}

/// Whether `inner` is what the innermost `block_on` the calling thread is inside drives, and so
/// what that call takes the seat of on the next turn of its loop. For a thread inside one.
fn drives_next(inner: &Arc<Core<Shared>>) -> bool {
    match innermost() {
        Target::Own => is_own(inner),
        Target::Core(core) => Arc::ptr_eq(&core, inner),
    }
}

/// The runtime a `block_on` is to drive, looked up afresh at each turn of its loop, because the
/// future it polls may be the very thing that brings the runtime into being. Asked on the
/// calling thread alone, so a lookup in that thread's own registry is what it is.
pub(crate) type Resolve<'a> = &'a dyn Fn() -> Option<Arc<Core<Shared>>>;

/// Runs `future` to completion on the calling thread, running the runtime `resolve` names
/// alongside it whenever the seat is free. `target` says which runtime that is, for what is built
/// or handed over on this thread while the call lasts.
///
/// Panics when called from a thread that is in the seat already, which is a call from inside a
/// task the runtime is running: such a call could only wait for the thread it is on.
pub(crate) fn block_on<F>(target: Target, resolve: Resolve<'_>, future: F) -> F::Output
where
    F: Future,
{
    let _inside = InBlockOn::enter(target);
    // The seat outlives the future, which [`drive`] drops as it returns: whatever that future
    // held — a task, a registration, a timer — is gone before the seat is given up, so only
    // work that outlives this call is handed on to a helper.
    let mut leaving = Leaving {
        resolve,
        driving: None,
    };

    drive(resolve, &mut leaving.driving, future)
}

/// What a `block_on` leaves as it goes: the seat it took, where it took one, and the work it was
/// running with nobody to run it.
///
/// A guard rather than a step after the poll loop, so that a panic out of the future is seen to
/// exactly as a return is. A future that panicked may have spawned tasks, registered sources or
/// armed timers that outlive it, and what drives those from here on is decided here.
struct Leaving<'a> {
    resolve: Resolve<'a>,
    /// The seat this call took, where it took one, given up as this is dropped.
    driving: Option<Driving>,
}

impl Drop for Leaving<'_> {
    /// Gives the seat up and hands on what the call leaves behind.
    ///
    /// A thread inside `block_on` asks for no helper for the runtime that call drives, on the
    /// promise that it takes the seat on the next turn of its loop and runs the work itself. A
    /// thread that leaves without ever having taken the seat has no next turn to keep that
    /// promise on, so what its last poll handed over is handed on here; one that had the seat
    /// did as much where it gave it up.
    ///
    /// The thread is taken out of the list of those waiting for the seat either way: where it
    /// had the seat, freeing it empties that list; where it had not, it is taken out here.
    fn drop(&mut self) {
        if self.driving.take().is_some() {
            return;
        }
        let Some(inner) = (self.resolve)() else {
            return;
        };
        stop_waiting_for(&inner);
    }
}

/// Takes the calling thread, which is not in `inner`'s seat, out of the list of those waiting
/// for it, and leaves the work on `inner` to whoever else can run it: what a `block_on` that
/// never took the seat does as it leaves, or as it is kept from its loop by a `block_on` nested
/// inside its future.
///
/// The helper gives the seat up to a call that asks for it and parks; a call that asks and then
/// stops asking without ever taking the seat up leaves nobody in it, and the helper is the one to
/// rouse. Where no helper is up at all, the work this thread asked none for — on the promise that
/// it would take the seat on the next turn of its loop — is handed on to one.
fn stop_waiting_for(inner: &Arc<Core<Shared>>) {
    let mut seat = seat(inner);
    seat.stop_waiting();
    if matches!(seat.holder, Holder::Nobody { .. }) {
        seat.rouse_helper();
    }
    hand_over(inner, &mut seat);
}

/// Starts the helper for `inner`, where it has a seat, unless a thread is in the seat or about to
/// take it. Called after the work it is to see is in place (a task queued, a source registered, a
/// deadline stored), never before.
///
/// A runtime with no seat is run by the threads inside `block_on` on it alone: it has no helper
/// to start.
///
/// That order is what makes the hand-off safe either way round: a helper that starts here finds
/// the work, and a thread in the seat either sees it in the round it is in or finds it where it
/// decides whether to leave, which it does under this very lock. A thread inside `block_on`
/// takes the seat on the next turn of its loop and finds the work then.
///
/// On WASI, where [`spawn_helper`] starts nothing, this starts no helper either, and the work waits
/// for the next `block_on` on `inner`.
pub(crate) fn ensure_helper(inner: &Arc<Core<Shared>>) {
    let Some(seat) = &inner.seat else {
        return;
    };
    // A thread in the seat of this very runtime — a `block_on` spawning from its future, or a task
    // it or the helper is running — finds the work in the round it is in, or where it decides
    // whether to leave: the same answer the lock below would give, for one thread-local read.
    if drives(&inner.remote) {
        return;
    }
    // A thread inside `block_on` on the very runtime the work is handed to asks for no helper: it
    // takes the seat on the next turn of its loop and runs the work itself. Work handed to any
    // other runtime — another thread's, or this thread's own while its innermost call drives a
    // different one — gets a helper as from any thread outside `block_on`, because this thread
    // is not about to sit in that seat.
    if in_block_on() && drives_next(inner) {
        return;
    }
    let mut seat = lock(seat);
    // Whoever is in the seat finds the work, and so does a helper parked with the seat left to
    // nobody, which was roused as the seat was left: by the call that freed it, or by the last
    // call waiting for it as that call stopped waiting — as it left, or as a `block_on` nested
    // inside its future took it away from its loop.
    if !matches!(
        seat.holder,
        Holder::Nobody {
            parked_helper: None
        }
    ) {
        return;
    }
    spawn_helper(inner, &mut seat);
}

/// Who is running a runtime, and who is waiting to.
///
/// Public in name only, in a module nobody outside can reach, because the sealed trait behind
/// [`Mode`](crate::Mode) names it.
pub struct Seat {
    holder: Holder,
    /// The threads parked in `block_on` for want of the seat, unparked whenever it is freed.
    ///
    /// Keyed by thread, so that a thread which enters `block_on` again and again while another
    /// keeps the seat has one place here rather than one per call. An entry goes where its
    /// thread takes the seat or leaves `block_on`, so that what is held here is the threads
    /// there may be something to unpark, and no more.
    waiting: HashMap<ThreadId, Thread>,
}

impl Seat {
    /// A seat nobody is in.
    pub(crate) fn new() -> Self {
        Self {
            holder: Holder::Nobody {
                parked_helper: None,
            },
            waiting: HashMap::new(),
        }
    }

    /// Whether the helper thread is up, in the seat or parked.
    #[cfg(test)]
    pub(crate) fn helper_running(&self) -> bool {
        matches!(self.holder, Holder::Helper) || self.parked_helper().is_some()
    }

    /// Whether the helper thread is up and parked, having left the seat to a `block_on`.
    ///
    /// Only tests that wait for an `Event` ask this, so it is built with the `event` feature.
    #[cfg(all(test, feature = "event"))]
    pub(crate) fn helper_parked(&self) -> bool {
        self.parked_helper().is_some()
    }

    /// How many threads are down as waiting for the seat.
    ///
    /// Only tests that wait for an `Event` ask this, so it is built with the `event` feature.
    #[cfg(all(test, feature = "event"))]
    pub(crate) fn waiting(&self) -> usize {
        self.waiting.len()
    }

    /// Takes the calling thread out of the list of those waiting for the seat.
    fn stop_waiting(&mut self) {
        self.waiting.remove(&thread::current().id());
    }

    /// Frees the seat, and unparks every thread waiting for it: to take it, or to find its
    /// future done. A helper that parked for want of it is roused too, a freed seat being the
    /// very thing it parked for.
    fn free(&mut self) {
        let parked_helper = match &mut self.holder {
            Holder::Nobody { parked_helper } | Holder::BlockOn { parked_helper } => {
                parked_helper.take()
            }
            Holder::Helper => None,
        };
        self.holder = Holder::Nobody { parked_helper };
        // Cleared here, which every way out of the seat comes through: a record left standing
        // would send what is spawned on this thread to a runtime it no longer runs, and, having
        // no destructor, would never let go of that runtime's allocation as the thread ends.
        set_driving::<Shared>(None);
        for (_, thread) in self.waiting.drain() {
            thread.unpark();
        }
        self.rouse_helper();
    }

    /// Rouses the helper where it is parked, so that it looks at the seat again: to take it
    /// back where work is left, and to leave where none is.
    ///
    /// The helper parks for the sake of a `block_on` that wants the seat, so whoever takes the
    /// last such call away — by freeing the seat, or by leaving the list of those waiting for
    /// it empty with nobody in it — owes the helper this.
    fn rouse_helper(&self) {
        if let Some(thread) = self.parked_helper() {
            thread.unpark();
        }
    }

    /// The helper thread, where it is parked with the seat left to a `block_on`.
    fn parked_helper(&self) -> Option<&Thread> {
        match &self.holder {
            Holder::Nobody { parked_helper } | Holder::BlockOn { parked_helper } => {
                parked_helper.as_ref()
            }
            Holder::Helper => None,
        }
    }
}

/// Who is in the seat, and the helper thread where it is parked outside it.
enum Holder {
    /// Nobody. The parked helper, if there is one, gave the seat up to a `block_on` that has
    /// not taken it yet, or that took it and already left again; it is roused to look at the
    /// seat once more.
    Nobody { parked_helper: Option<Thread> },
    /// A thread inside `block_on`, with the helper parked behind it if the helper is up.
    BlockOn { parked_helper: Option<Thread> },
    /// The helper thread.
    Helper,
}

/// The kind of thread a [`Driving`] belongs to. Unlike [`Holder`], it cannot be nobody: a guard
/// exists only while its thread is in the seat.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Driver {
    /// A thread inside `block_on`.
    BlockOn,
    /// The helper thread.
    Helper,
}

/// Polls `future` to completion, running the runtime `resolve` names alongside it whenever the
/// seat is free, and leaving in `driving` the seat it took, where it took one.
///
/// The future is this call's own, and is dropped where it returns, before the caller gives the
/// seat up.
fn drive<F>(resolve: Resolve<'_>, driving: &mut Option<Driving>, future: F) -> F::Output
where
    F: Future,
{
    let mut future = pin!(future);
    let signal = Arc::new(Signal {
        thread: thread::current(),
        woken: AtomicBool::new(false),
        runtime: Mutex::new(Weak::new()),
    });
    let waker = Waker::from(signal.clone());
    let mut cx = Context::from_waker(&waker);
    let take_seat = |driving: &mut Option<Driving>| {
        *driving = resolve().and_then(|inner| Driving::take(inner, Driver::BlockOn));
        // Set before the round that follows, so that a wake arriving during its wait finds the
        // runtime whose wait to break.
        if let Some(driving) = driving {
            *lock(&signal.runtime) = Arc::downgrade(&driving.inner);
        }
    };
    let mut failed_waits = 0u32;
    loop {
        if driving.is_none() {
            take_seat(driving);
        }
        // Lowered before the poll, so that a wake during it is seen by the round or the park
        // that follows, and with a swap, so that the poll sees what the wakes before it were
        // for.
        signal.woken.swap(false, Ordering::AcqRel);
        // Budgeted like a task, because the tasks run only between its polls: see
        // `Core::block_on`.
        if let Poll::Ready(output) = coop::budgeted(|| future.as_mut().poll(&mut cx)) {
            return output;
        }
        // Looked for again, because the poll may have been what brought the runtime into being,
        // and a thread inside `block_on` asks for no helper: where nobody else is in the seat,
        // the work that poll left behind is this thread's to run.
        if driving.is_none() {
            take_seat(driving);
        }
        match &*driving {
            Some(driving) => driving.round(&signal.woken, &mut failed_waits),
            // On WASI without threads, a park returns at once, and the next turn polls again. No
            // seat is ever found taken there: no helper is up, and the one thread there is would
            // have been turned away on entering this call, had it been in the seat. So this is
            // reached only while `resolve` names no runtime, and the turn after the poll that
            // brings one into being takes its seat. A future that waits meanwhile for what no
            // poll of it and no runtime this thread drives can bring, such as a task of a runtime
            // nobody is inside `block_on` on, waits for good, as a park would have it wait with
            // nobody to wake it, though it spins rather than sleeps.
            None => {
                if !signal.woken.load(Ordering::Acquire) {
                    thread::park();
                }
            }
        }
    }
}

/// Starts a helper for what is left to run, watch or time on `inner`, unless a thread is in the
/// seat or a helper is up already. Under the seat lock, which `seat` is the guard of.
///
/// Unlike [`ensure_helper`], this pays no heed to the caller being inside `block_on`: it is for
/// the places where a thread that was running the work, or was about to, stops being able to.
///
/// On WASI, where [`spawn_helper`] starts nothing, the work is left for the next `block_on` on
/// `inner`.
fn hand_over(inner: &Arc<Core<Shared>>, seat: &mut Seat) {
    if !matches!(
        seat.holder,
        Holder::Nobody {
            parked_helper: None
        }
    ) || !inner.is_busy()
    {
        return;
    }
    spawn_helper(inner, seat);
}

/// Starts the helper thread. Under the seat lock, which `seat` is the guard of.
///
/// On WASI, starts nothing and leaves the seat free: no helper is ever up there (see the module
/// documentation), so the work waits for the next `block_on` on `inner`.
fn spawn_helper(inner: &Arc<Core<Shared>>, seat: &mut Seat) {
    // The one place a helper is started, and so the one place it is kept from starting. A `cfg!`
    // rather than a `#[cfg]`, so that the helper's side of the seat is still built on WASI, never
    // run, rather than having each item of it left out there one by one.
    if cfg!(target_os = "wasi") {
        return;
    }
    let inner = inner.clone();
    thread::Builder::new()
        .name(THREAD_NAME.into())
        .spawn(move || helper(inner))
        .expect("the thread a runtime's helper runs on");
    // The seat is the helper's from here, under the lock this is called with, so there is no
    // window in which the thread is on its way and the seat looks free to a second spawn or to a
    // `block_on`. Set once the thread is there: a spawn that fails panics with the seat free, so
    // that the next call tries again rather than wait on a thread that was never started.
    seat.holder = Holder::Helper;
}

/// The name the helper thread carries: eight bytes, which fits the fifteen Linux keeps for one.
const THREAD_NAME: &str = "zruntime";

/// What the helper thread does: the seat, and rounds until nothing is left.
///
/// Each round polls what is ready, a batch at a time, and then asks whether anything is left to
/// run, watch or time. Where nothing is, the thread leaves there and then, rather than sit in a
/// wait until something comes along to tell it what it could have worked out for itself. Where
/// something is, the round ends in one wait on the reactor — unless a `block_on` is waiting for
/// the seat, in which case the seat is that call's and this thread parks until it is free again.
fn helper(inner: Arc<Core<Shared>>) {
    let mut failed_waits = 0u32;
    // The seat was made this thread's by whoever started it.
    let mut driving = Some(Driving::enter(inner.clone(), Driver::Helper));
    loop {
        if let Some(driving) = driving {
            loop {
                driving.run_batch();
                if driving.leave_if_idle() {
                    return;
                }
                if driving.yield_if_wanted() {
                    break;
                }
                driving.wait(false, &mut failed_waits);
            }
        }
        // Both ways of getting here put this thread down as parked, with the seat a
        // `block_on`'s to take or to hand back, and that call is what ends the park. A park
        // that ends of its own accord costs no more than another look at the seat.
        thread::park();
        driving = Driving::take(inner.clone(), Driver::Helper);
    }
}

/// A thread's time in the seat: taken here, given up when this is dropped.
struct Driving {
    inner: Arc<Core<Shared>>,
    who: Driver,
    /// Whether the seat has been given up already, which [`Driving::leave_if_idle`] and
    /// [`Driving::yield_if_wanted`] are the two things that do before the drop.
    left: Cell<bool>,
}

impl Driving {
    /// Takes the seat as `who` if it is free.
    ///
    /// A `block_on` that finds it taken is put down as waiting, to be unparked when the seat is
    /// freed, and tells the helper where the helper is the one in it: the helper looks for
    /// waiters between a batch and a wait, so the notification is what brings its wait to an
    /// end. A `block_on` in the seat frees it when its future is done and needs no telling.
    ///
    /// A helper that finds the seat taken parks rather than leaves, and puts itself down as
    /// parked under the very lock the seat's holder frees the seat under: whoever is in the
    /// seat rouses it as it goes.
    fn take(inner: Arc<Core<Shared>>, who: Driver) -> Option<Self> {
        let mut seat = seat(&inner);
        match (&mut seat.holder, who) {
            (Holder::Nobody { parked_helper }, Driver::BlockOn) => {
                let parked_helper = parked_helper.take();
                seat.holder = Holder::BlockOn { parked_helper };
                // A turn of this call's loop before this one may have found the seat taken and
                // put the thread down as waiting for it, which a thread in the seat is not.
                seat.stop_waiting();
            }
            // The parked thread, if any, was this very helper.
            (Holder::Nobody { .. }, Driver::Helper) => seat.holder = Holder::Helper,
            (Holder::BlockOn { parked_helper }, Driver::Helper) => {
                *parked_helper = Some(thread::current());

                return None;
            }
            // One helper runs per runtime, so a helper cannot find another already there.
            (Holder::Helper, Driver::Helper) => {
                unreachable!("a second helper for the same runtime")
            }
            (holder, Driver::BlockOn) => {
                let helper_in_seat = matches!(holder, Holder::Helper);
                let thread = thread::current();
                seat.waiting.insert(thread.id(), thread);
                if helper_in_seat {
                    drop(seat);
                    inner.remote.notify();
                }

                return None;
            }
        }
        drop(seat);

        Some(Self::enter(inner, who))
    }

    /// Marks the calling thread as the one in the seat of `inner`, which the caller has made it:
    /// [`Driving::take`] under the seat lock, or the spawn of the helper thread.
    fn enter(inner: Arc<Core<Shared>>, who: Driver) -> Self {
        // Written down on a thread whose locals are being destroyed too, which reaches here from
        // the destructor of one of them, in an order that is not this runtime's to choose: the
        // record has no destructor, so it is there to write to whatever that order, and what is
        // built in the seat goes on this runtime there as anywhere else.
        set_driving::<Shared>(Some(&inner));

        Self {
            inner,
            who,
            left: Cell::new(false),
        }
    }

    /// Polls up to a batch of ready tasks.
    fn run_batch(&self) {
        self.inner.run_batch();
    }

    /// One round for `block_on`: a batch, then one wait on the reactor, which waits for nothing
    /// where the future this thread is polling has been woken in the meantime.
    fn round(&self, woken: &AtomicBool, failed_waits: &mut u32) {
        self.inner.round(woken, failed_waits);
    }

    /// One wait on the reactor: bounded by no time at all where a task is ready or `at_once`
    /// says the caller has something of its own to get back to, and by nothing where neither
    /// holds.
    fn wait(&self, at_once: bool, failed_waits: &mut u32) {
        self.inner.wait(at_once, failed_waits);
    }

    /// Gives the seat up if nothing is left to run, watch or time; what the helper does before
    /// each wait. Under the seat lock, so that a spawn or a registration racing with it either
    /// is seen here or starts a helper itself once the lock is released.
    fn leave_if_idle(&self) -> bool {
        let mut seat = seat(&self.inner);
        if self.inner.is_busy() {
            return false;
        }
        seat.free();
        self.left.set(true);

        true
    }

    /// Gives the seat up to the `block_on` calls waiting for it and puts this thread down as
    /// parked; what the helper asks before each wait. `true` where it did, and the caller parks.
    ///
    /// Freed first, with the holder still `Helper` and so no parked helper for [`Seat::free`] to
    /// rouse, and marked parked after: a rouse of this very thread would only cut short the park
    /// it is about to make.
    fn yield_if_wanted(&self) -> bool {
        let mut seat = seat(&self.inner);
        if seat.waiting.is_empty() {
            return false;
        }
        seat.free();
        seat.holder = Holder::Nobody {
            parked_helper: Some(thread::current()),
        };
        self.left.set(true);

        true
    }
}

impl Drop for Driving {
    /// Gives the seat up, unless [`Driving::leave_if_idle`] or [`Driving::yield_if_wanted`] did
    /// already, which is what the flag this reads says: the seat may be somebody else's by now,
    /// and what is asked here is whether it is still this thread's to give up, not who is in it.
    ///
    /// A `block_on` that leaves work behind starts a helper for it, because whatever is left
    /// may be awaited by a thread that is not going to drive; a helper that unwinds out of the
    /// loop puts itself down as gone, so that the next spawn, registration or timer poll starts
    /// another.
    fn drop(&mut self) {
        if self.left.get() {
            return;
        }
        let mut seat = seat(&self.inner);
        seat.free();
        if self.who == Driver::BlockOn {
            hand_over(&self.inner, &mut seat);
        }
    }
}

/// The waker of the future a `block_on` polls.
struct Signal {
    thread: Thread,
    /// Whether the future has been woken since it was last polled.
    ///
    /// Raised by each wake and lowered before each poll, both with `AcqRel` swaps rather than
    /// stores. The lowering acquires every wake since the one before it, so that the poll sees
    /// what they were for: a store reads nothing, and a store that raised the flag would cut the
    /// wakes before it off from the lowering. A wake that comes after a lowering acquires it in
    /// turn, and with it the end of the reactor wait before it, so that the `notify` the wake
    /// makes cannot take the wake-up that wait took out for one still to come.
    woken: AtomicBool,
    /// The runtime the thread is in the seat of, set by that thread as it takes the seat: a wake
    /// may come from any thread, and only the driving thread knows which runtime's wait it is in.
    runtime: Mutex<Weak<Core<Shared>>>,
}

impl Wake for Signal {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    /// Marks the future woken and rouses its thread, wherever that thread is: parked, or in the
    /// seat inside the reactor's wait, which is what the notification ends. A wake from the
    /// thread itself writes no notification, because that thread looks at the flag before its
    /// next wait; a wake of a thread in no seat needs none, because that thread is parked.
    fn wake_by_ref(self: &Arc<Self>) {
        self.woken.swap(true, Ordering::AcqRel);
        self.thread.unpark();
        // Bound first, so that the lock is let go of before the runtime is reached: the handle
        // taken here may be the last one, and the runtime it drops wakes whatever it held.
        let runtime = lock(&self.runtime).upgrade();
        if let Some(inner) = runtime {
            inner.remote.notify();
        }
    }
}

/// What says a thread is inside `block_on`, and what that call drives, for as long as it is.
struct InBlockOn {
    /// Whether the call's target went on the thread's record, which it cannot once that record
    /// is gone, on a thread whose locals are being destroyed.
    recorded: bool,
    /// What the call this one is nested in held in [`HELD`], put back as this one leaves.
    outer_held: Option<Arc<Core<Shared>>>,
}

impl InBlockOn {
    /// Marks the calling thread as inside a `block_on` that drives `target`.
    ///
    /// Panics where the thread is in a seat already: the call comes from inside a task the
    /// runtime is running on this thread, and could only ever wait for itself.
    fn enter(target: Target) -> Self {
        assert!(
            !drives_a_runtime(),
            "block_on called from a task this runtime is running: the call would wait for the \
             very thread it is on"
        );
        if in_block_on() {
            leave_the_outer_call(&target);
        }
        IN_BLOCK_ON.with(|inside| inside.set(inside.get() + 1));
        let recorded = TARGETS
            .try_with(|targets| targets.borrow_mut().push(target))
            .is_ok();
        // Taken out rather than left for this call to share, so that what this call holds goes
        // as it leaves, and the outer call's hold is there again once it has.
        let outer_held = HELD.with(|held| held.borrow_mut().take());

        Self {
            recorded,
            outer_held,
        }
    }
}

/// Takes a thread about to enter a `block_on` nested inside the future of another off the list
/// of those waiting for the seat the outer call drives, where the two drive different runtimes.
/// For a thread inside a `block_on` and in no seat.
///
/// The outer call is in no seat, or the check before this would have turned the nested one away,
/// so it may be down as waiting for its seat, with the helper about to give the seat up to it and
/// park. It cannot take the seat up while the nested call has the thread, so the helper would be
/// left parked with the seat free and nobody to rouse it: a spawn on that runtime from anywhere
/// takes a parked helper for one somebody is about to rouse, and asks for no other. Taken off the
/// list, the thread leaves the helper in the seat, or rouses it where it parked already; and work
/// it handed that runtime before the nested call, asking for no helper on the promise of the next
/// turn of its loop, is handed on to one where none is up. The outer call puts itself down again
/// at that next turn, where it asks for the seat afresh.
///
/// A nested call on the very runtime the outer one drives waits for, or takes, that same seat, so
/// the thread keeps its place in the list and the promise stands.
fn leave_the_outer_call(target: &Target) {
    // Both looked up before any seat lock is taken, because the thread's own registry is behind a
    // lock of its own; and held until that lock is let go of, because either may be the last
    // handle on its runtime.
    let outer = match innermost() {
        Target::Own => own(),
        Target::Core(core) => Some(core),
    };
    let Some(outer) = outer else {
        return;
    };
    let nested = match target {
        Target::Own => own(),
        Target::Core(core) => Some(core.clone()),
    };
    if nested.is_some_and(|nested| Arc::ptr_eq(&nested, &outer)) {
        return;
    }

    stop_waiting_for(&outer);
}

impl Drop for InBlockOn {
    fn drop(&mut self) {
        if self.recorded {
            let popped = TARGETS.try_with(|targets| targets.borrow_mut().pop());
            // Clear of the borrow: the handle it holds may be the last one on its runtime.
            drop(popped);
        }
        let outer_held = self.outer_held.take();
        let held = HELD.with(|held| mem::replace(&mut **held.borrow_mut(), outer_held));
        // Clear of the borrow, for the same reason.
        drop(held);
        IN_BLOCK_ON.with(|inside| inside.set(inside.get() - 1));
    }
}

thread_local! {
    /// How many `block_on` calls this thread is inside of, in the seat or waiting for it. A
    /// count rather than a flag, so that one call returning does not unmark the call it was
    /// made from.
    ///
    /// Kept beside [`TARGETS`] rather than read off its length: a count needs no destructor, so
    /// it outlives every other local of the thread and says so even for a call made from the
    /// destructor of one of them.
    static IN_BLOCK_ON: Cell<u32> = const { Cell::new(0) };

    /// What each `block_on` this thread is inside of drives, the innermost last: what is built on
    /// this thread goes on the runtime the innermost call drives, and work handed to that runtime
    /// needs no helper while the call lasts.
    static TARGETS: RefCell<Vec<Target>> = const { RefCell::new(Vec::new()) };

    /// The thread's own runtime, where a free `spawn` inside the innermost `block_on` this thread
    /// is inside of brought it into being, held by that call until it returns: nothing else holds
    /// it, and the call takes its seat only on the next turn of its loop (see
    /// [`current_to_spawn_on`]). Each call takes what the call around it holds out of here as it
    /// enters, and puts it back as it leaves, so that what is here is the innermost call's alone.
    ///
    /// With no destructor, unlike [`TARGETS`], so that it is there for a `block_on` run from the
    /// destructor of another of the thread's locals, whatever order they go in. Its value is kept
    /// in a `ManuallyDrop` for that, which leaks nothing: a thread cannot end inside `block_on`,
    /// and the outermost call puts back the `None` that was here before it.
    static HELD: RefCell<ManuallyDrop<Option<Arc<Core<Shared>>>>> =
        const { RefCell::new(ManuallyDrop::new(None)) };
}

/// The lock around `core`'s seat.
///
/// For a runtime the calling code knows has one: every runtime a `block_on` here resolves to or
/// a helper runs came out of a registry, or was made with a seat of its own.
pub(crate) fn seat(core: &Core<Shared>) -> MutexGuard<'_, Seat> {
    let Some(seat) = &core.seat else {
        unreachable!("a runtime the seat machinery runs has a seat");
    };

    lock(seat)
}

impl Core<Shared> {
    /// Whether anything is left to run, watch or time.
    pub(crate) fn is_busy(&self) -> bool {
        self.remote.has_ready() || self.scheduler.live_tasks() > 0 || !self.reactor.is_idle()
    }
}

/// Whether `inner` is the calling thread's own runtime, the one the free `block_on` drives.
///
/// Nothing is a thread's own once its locals are gone, so work handed over then gets a helper,
/// as work handed to any runtime the caller does not drive does.
fn is_own(inner: &Arc<Core<Shared>>) -> bool {
    OWN.try_with(|own| std::ptr::eq(lock(own).as_ptr(), Arc::as_ptr(inner)))
        .unwrap_or(false)
}

/// The runtime `registry` names, made here if none is alive.
pub(crate) fn shared_in(registry: &Mutex<Weak<Core<Shared>>>) -> io::Result<Arc<Core<Shared>>> {
    let mut shared = lock(registry);
    if let Some(core) = shared.upgrade() {
        return Ok(core);
    }
    let core = Core::with_seat()?;
    *shared = Arc::downgrade(&core);

    Ok(core)
}

/// Makes `core` the calling thread's own runtime until the value handed back is dropped, which
/// puts back whatever was there before, whether the call returns or unwinds.
///
/// For a test that drives a runtime of its own making: the runtime a `block_on` resolves to is
/// always the one its thread builds on, so a spawn from inside such a call asks for no helper,
/// and a test that drives a runtime nobody's registry names would be told otherwise.
#[cfg(test)]
pub(crate) fn own_for_the_call(core: &Arc<Core<Shared>>) -> impl Drop {
    struct Restore(Weak<Core<Shared>>);

    impl Drop for Restore {
        fn drop(&mut self) {
            OWN.with(|own| *lock(own) = std::mem::take(&mut self.0));
        }
    }

    OWN.with(|own| Restore(std::mem::replace(&mut *lock(own), Arc::downgrade(core))))
}

thread_local! {
    /// This thread's runtime, if one is alive: the one a `block_on` on this thread drives, and
    /// the one work built inside such a call goes on.
    ///
    /// A `Weak`, so that the runtime and the two descriptors its reactor holds go once the last
    /// handle and any thread running it are gone, and the next handle brings a fresh one. A
    /// thread that ends with work alive leaves it to the helper thread that took it over when
    /// its last `block_on` returned.
    static OWN: Mutex<Weak<Core<Shared>>> = const { Mutex::new(Weak::new()) };
}

/// The runtime for work built on a thread that is inside no `block_on` and in no seat: work some
/// other executor polls, wherever in the process it is built.
///
/// Such work has no thread of its own to look to, so a helper runs it, and one runtime for all
/// of it is one helper and one pair of descriptors rather than a set per thread. A `Weak`, for
/// the same reason [`OWN`] is.
static SHARED: Mutex<Weak<Core<Shared>>> = Mutex::new(Weak::new());
