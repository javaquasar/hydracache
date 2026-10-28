//! The wait everywhere but on Apple's platforms and the BSDs: blocking work on a thread of the pool
//! kept for the waits for children, and, on Linux where the system gives one, a pidfd that the
//! runtime watches instead.
//!
//! The work is a thread held for as long as the child runs, so it is started by the first wait for
//! the exit rather than by the child's creation, and a child that nobody waits for holds none. It
//! runs on the pool that a dropped child is reaped on as well, and not on that of
//! [`unblock()`](crate::unblock()), which a program that waits for many long-running children at
//! once would otherwise fill, for all its other blocking work to wait behind.

#[cfg(target_os = "linux")]
use std::os::fd::OwnedFd;
#[cfg(windows)]
use std::os::windows::io::{AsHandle, AsRawHandle};
use std::{io, marker::PhantomData};

#[cfg(target_os = "linux")]
use rustix::process::{PidfdFlags, pidfd_open};
#[cfg(unix)]
use rustix::{
    io::Errno,
    process::{Pid, WaitId, WaitIdOptions, waitid},
};
#[cfg(windows)]
use windows_sys::Win32::{
    Foundation::WAIT_FAILED,
    System::Threading::{INFINITE, WaitForSingleObject},
};

#[cfg(target_os = "linux")]
use crate::AsyncIo;
use crate::{BlockingWork, Mode, Runtime, process::WAITS, unblock::unblock_on};

/// What tells a [`Child`](crate::process::Child) that its process has exited: see the
/// [module documentation](super) of what every platform's does.
pub(crate) enum Exit<M>
where
    M: Mode,
{
    /// A pidfd of the process, which the runtime watches until it reports the exit.
    ///
    /// Nothing is done with it but waiting for it to be readable. The readiness is level-triggered:
    /// the descriptor stays readable once the process has exited, so a wait that is given up on and
    /// made again finds it as it was.
    ///
    /// A pidfd turns readable as the process exits, which is when its parent can collect it,
    /// unless another process traces it: the tracer then has the process first, for as long as it
    /// takes, and the descriptor is readable all that while. So once it has reported the exit, it
    /// has nothing more to tell, and is let go of: a wait after that, for a process that still
    /// cannot be collected, goes on on the pool, whose `waitid` waits for it without spinning.
    #[cfg(target_os = "linux")]
    Watched(AsyncIo<OwnedFd, M>),
    /// Blocking work on a thread of the pool kept for the waits for children.
    Pool {
        /// The blocking work that waits, from the first wait until it has resolved.
        ///
        /// Kept across a wait that is given up, so that the next one takes up the same work
        /// instead of starting another thread's worth of it.
        work: Option<BlockingWork<io::Result<()>>>,
        /// Ties the type to its runtime's flavour, whose pointer a local runtime's handles cannot
        /// send to another thread, as the pipes of the child cannot be sent either.
        _mode: PhantomData<M::Ptr<()>>,
    },
}

impl<M> Exit<M>
where
    M: Mode,
{
    /// The wait for `child`, a process spawned on `runtime`, to exit, through a pidfd that
    /// `runtime` watches, if the system gives one.
    ///
    /// A system that does not, as Linux before 5.3 does not, or that turns the call away, as the
    /// seccomp profile of an older container may, is no reason to fail: the wait runs on the pool
    /// instead. Nothing waits yet in either case; the first call of [`wait`](Exit::wait) does, and
    /// on the pool it is that call that starts the work.
    ///
    /// Android is left out, though its kernel has the call: its seccomp policy kills a process that
    /// makes a system call it does not allow, as older versions do for this one, rather than fail
    /// the call with an error to fall back from.
    ///
    /// What can fail is the runtime taking the pidfd under its watch.
    #[cfg(target_os = "linux")]
    pub(crate) fn new(runtime: &Runtime<M>, child: &std::process::Child) -> io::Result<Self> {
        let Ok(pidfd) = pidfd_open(Pid::from_child(child), PidfdFlags::empty()) else {
            return Ok(Self::pool());
        };

        // Registered as it is, rather than through `AsyncIo::new`, which would switch it to
        // non-blocking mode first: nothing is ever read from or written to a pidfd here, only its
        // readiness is waited for, so its mode does not matter.
        Ok(Self::Watched(AsyncIo::from_nonblocking(runtime, pidfd)?))
    }

    /// The wait for `child`, a process spawned on `runtime`, to exit, not started yet.
    ///
    /// Nothing is started here: the first call of [`wait`](Exit::wait) starts the work, which gets
    /// the child from that call, so neither the runtime nor the child is used yet. The result is
    /// there for a wait that has something to set up, and can fail at it. This one has nothing,
    /// and never fails.
    #[cfg(not(target_os = "linux"))]
    pub(crate) fn new(_runtime: &Runtime<M>, _child: &std::process::Child) -> io::Result<Self> {
        Ok(Self::pool())
    }

    /// Waits for `child`, the process this was made for, to exit, without collecting its status.
    ///
    /// Resolves once the process has exited, or to the error that stopped the wait: a child that
    /// some other code has collected the status of, say, or a pidfd that the system's poller
    /// refuses to watch. Dropping the future loses nothing, and leaves the next call to pick up
    /// where the dropped one left off: on the pool, the first call starts the work that waits,
    /// which keeps running, so that calls given up on do not pile up threads.
    pub(crate) async fn wait(&mut self, child: &std::process::Child) -> io::Result<()> {
        match self {
            #[cfg(target_os = "linux")]
            Self::Watched(pidfd) => {
                pidfd.readable().await?;
                *self = Self::pool();

                Ok(())
            }
            Self::Pool { work, .. } => {
                let running = match &mut *work {
                    Some(running) => running,
                    none @ None => none.insert(start(child)?),
                };
                let outcome = running.await;
                // Only once the work has resolved, so that a future dropped before then leaves it
                // for the next call.
                *work = None;

                outcome
            }
        }
    }

    /// The wait on the pool, which nothing has started yet.
    fn pool() -> Self {
        Self::Pool {
            work: None,
            _mode: PhantomData,
        }
    }
}

/// Blocking work that returns once `child` has exited, with the child left as it is.
///
/// `waitid` with `WNOWAIT` leaves the child waitable for the `try_wait` of std's `Child`, which
/// collects it: the work never reaps, so the process ID is not free to be reused for as long as
/// the child's owner holds the `Child`.
#[cfg(unix)]
fn start(child: &std::process::Child) -> io::Result<BlockingWork<io::Result<()>>> {
    let pid = Pid::from_child(child);

    Ok(unblock_on(&WAITS, move || {
        loop {
            match waitid(
                WaitId::Pid(pid),
                WaitIdOptions::EXITED | WaitIdOptions::NOWAIT,
            ) {
                // A call a signal interrupted is made again: the child has not exited yet.
                Err(Errno::INTR) => {}
                Ok(_) => return Ok(()),
                Err(error) => return Err(error.into()),
            }
        }
    }))
}

/// Blocking work that returns once `child` has exited.
///
/// The work holds a handle of its own on the process, which keeps it to wait on whatever becomes
/// of the `Child` it was made from: the work runs to its end whether or not anything waits for its
/// outcome.
#[cfg(windows)]
fn start(child: &std::process::Child) -> io::Result<BlockingWork<io::Result<()>>> {
    let handle = child.as_handle().try_clone_to_owned()?;

    Ok(unblock_on(&WAITS, move || {
        // SAFETY: `WaitForSingleObject` is given a handle that `handle`, which this closure owns,
        // keeps open for the call, and a timeout that has it wait for as long as it takes. It
        // reads and writes nothing else.
        let waited = unsafe { WaitForSingleObject(handle.as_raw_handle(), INFINITE) };
        if waited == WAIT_FAILED {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }))
}
