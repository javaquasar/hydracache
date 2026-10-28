//! The wait on Apple's platforms and the BSDs: a kqueue of the child's own, with a filter on the
//! process, whose descriptor the runtime watches.
//!
//! The filter reports the exit as an event, which the queue keeps, as nothing ever retrieves it:
//! the queue's descriptor turns readable once the process has exited, and stays so. Every poller
//! the reactor waits on there watches such a descriptor as it does a pipe: kqueue, which can watch
//! another queue, `select(2)` on Apple's platforms, and `poll(2)`.

use std::{io, mem::MaybeUninit, os::fd::OwnedFd, ptr, time::Duration};

use rustix::{
    event::kqueue::{Event, EventFilter, EventFlags, ProcessEvents, kevent, kqueue},
    io::{Errno, FdFlags, fcntl_setfd},
    process::Pid,
};

use crate::{AsyncIo, Mode, Runtime};

/// What tells a [`Child`](crate::process::Child) that its process has exited: see the
/// [module documentation](super) of what every platform's does.
pub(crate) enum Exit<M>
where
    M: Mode,
{
    /// The child's kqueue, which the runtime watches.
    ///
    /// Nothing is done with it but waiting for it to be readable. The readiness is level-triggered:
    /// the descriptor stays readable once the process has exited, so a wait that is given up on and
    /// made again, or made after the child has been collected, finds it as it was.
    Watched(AsyncIo<OwnedFd, M>),
    /// The process has exited, as far as a kqueue can tell: it was gone before it could be
    /// watched, or the queue has reported its exit already.
    ///
    /// Apple's kqueue turns the filter away (`ESRCH`) for a process that has exited and waits to be
    /// collected, as it does for one that does not exist. A queue reports the exit as the process
    /// starts to go, a moment before its status can be collected, on Apple's platforms and on
    /// FreeBSD at least. The status of a child is looked at before every wait for it, so a wait
    /// that finds this is for a process that has stopped running and is not yet there to be
    /// collected, which is how a process looks for that moment, while the system tears it down.
    /// Such a wait resolves after a pause, for the status to be looked at again, rather than at
    /// once, which would have the caller look again with no pause at all, and the thread that every
    /// task of the runtime shares with it. The runtime is held to time the pause.
    Exited(Runtime<M>),
}

impl<M> Exit<M>
where
    M: Mode,
{
    /// The wait for `child`, a process spawned on `runtime`, to exit, through a kqueue of the
    /// child's own that `runtime` watches.
    ///
    /// What can fail is the system making the kqueue, which it does not once the process has run
    /// out of descriptors, the filter being added to it, for any reason but the process being gone,
    /// and the runtime taking the kqueue under its watch. These platforms have no wait on the pool
    /// to fall back on, so each of them fails the spawn, as a pipe that the system cannot make
    /// does. Nothing waits yet; the first call of [`wait`](Exit::wait) does.
    pub(crate) fn new(runtime: &Runtime<M>, child: &std::process::Child) -> io::Result<Self> {
        let queue = kqueue()?;
        // Close-on-exec, as the descriptors of the runtime's own poller are: rustix makes the queue
        // with a plain `kqueue()`, which leaves it unset.
        fcntl_setfd(&queue, FdFlags::CLOEXEC)?;
        if !watch(&queue, Pid::from_child(child))? {
            return Ok(Self::Exited(runtime.clone()));
        }

        // Registered as it is, rather than through `AsyncIo::new`, which would switch it to
        // non-blocking mode first: nothing is ever read from or written to a queue here, only its
        // readiness is waited for, so its mode does not matter. Apple's platforms turn the switch
        // away anyway (`ENOTTY`), for a kqueue is no device that has a mode of its own.
        Ok(Self::Watched(AsyncIo::from_nonblocking(runtime, queue)?))
    }

    /// Waits for the process this was made for to exit, without collecting its status.
    ///
    /// Resolves once the process has exited, or to the error that stopped the wait: a kqueue that
    /// the system's poller refuses to watch. Dropping the future loses nothing, and the next call
    /// finds the descriptor as it was. The wait is on a descriptor of this process alone, which is
    /// why it has no use for the child.
    ///
    /// Once the queue has reported the exit, it has nothing more to tell, and stays readable for
    /// good: it is let go of, and a wait after that, for a process that is not there to be
    /// collected yet, pauses for a moment instead, as one for a process that was gone before it
    /// could be watched does. See [`Exited`](Exit::Exited).
    pub(crate) async fn wait(&mut self, _child: &std::process::Child) -> io::Result<()> {
        match self {
            Self::Watched(queue) => {
                queue.readable().await?;
                let runtime = queue.runtime();
                *self = Self::Exited(runtime);

                Ok(())
            }
            Self::Exited(runtime) => {
                runtime.sleep(RECHECK).await;

                Ok(())
            }
        }
    }
}

/// Has `queue` watch for the exit of the process `pid`, and tells whether the process was there
/// to be watched.
///
/// The filter is added with a receipt asked for, which the queue answers in place of an event: the
/// error number of the change, or zero where it was made. The queue then reports the exit as an
/// event of its own, once it happens, and keeps it, as nothing ever retrieves it: the queue's
/// descriptor stays readable from then on. `pid` is the ID of a child that nobody has collected,
/// so it is still the child's own: no other process can have been given it.
fn watch(queue: &OwnedFd, pid: Pid) -> io::Result<bool> {
    let change = Event::new(
        EventFilter::Proc {
            pid,
            flags: ProcessEvents::EXIT,
        },
        EventFlags::ADD | EventFlags::RECEIPT,
        ptr::null_mut(),
    );
    let mut receipt = [MaybeUninit::uninit(); 1];
    // SAFETY: rustix asks that every descriptor a change names stay valid for as long as the
    // queue lives. This change names none: a process filter is keyed by a process ID, not by a
    // descriptor, and the udata is null, which neither the kernel nor this function dereferences.
    // `queue` is open for the call, and the call has room for the one receipt that comes back and
    // a timeout of zero, so that it never waits.
    let (receipts, _) = unsafe { kevent(queue, &[change], &mut receipt, Some(Duration::ZERO)) }?;
    debug_assert_eq!(receipts.len(), 1, "the change comes back as a receipt");
    // The data of an event is an error number only where `EV_ERROR` is set, as it is on every
    // receipt.
    let Some(receipt) = receipts
        .first()
        .filter(|receipt| receipt.flags().contains(EventFlags::ERROR))
    else {
        return Ok(true);
    };

    match receipt.data() {
        0 => Ok(true),
        // The kernel writes an `int` there, which an `i32` holds whole.
        errno => match Errno::from_raw_os_error(errno as i32) {
            // The process has exited and waits to be collected, or never was.
            Errno::SRCH => Ok(false),
            errno => Err(errno.into()),
        },
    }
}

/// How long a wait for a process that has exited, and cannot be collected yet, pauses for.
///
/// A process that is gone and cannot be collected yet is one the system is still tearing down,
/// which takes a moment, so the pause is short. It is a pause all the same, so that looking at the
/// status again and again does not keep the thread from the other tasks for as long as that lasts.
const RECHECK: Duration = Duration::from_millis(1);
