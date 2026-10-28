//! The wait on Linux and Android: epoll, which keeps what it watches in the kernel, and a pipe that
//! breaks the wait.
//!
//! The kernel holds every source watched in a direction, in the directions it was last told to
//! watch it in, and sees each change as it is made. A wait hands nothing over and asks only for
//! the sources the kernel found ready, so that it costs what those cost, however many more are
//! watched. The read end of the pipe is watched beside them, under a token of its own.
//!
//! A source is added to the epoll instance as it gains its first direction, and deleted from it as
//! it loses its last. epoll reports an error or a hang-up of a descriptor whatever it watches it
//! for, so a source watched for nothing that hung up would end every wait at once if it were in
//! there; and a source nobody ever waits on costs no system call.
//!
//! epoll takes its own timeout in whole milliseconds, rounded up, which would have a timer fire up
//! to a millisecond late. A wait with a timeout sets a timerfd, which takes nanoseconds and which
//! the instance watches as well, and has epoll wait without one.

use std::{
    io,
    mem::MaybeUninit,
    os::fd::{BorrowedFd, OwnedFd},
    time::Duration,
};

use rustix::{
    event::{
        Timespec,
        epoll::{self, CreateFlags, Event, EventData, EventFlags},
    },
    io::{Errno, read},
    time::{
        Itimerspec, TimerfdClockId, TimerfdFlags, TimerfdTimerFlags, timerfd_create,
        timerfd_settime,
    },
};

use super::{Directions, RawSource, Ready, pipe::Pipe};

pub(crate) struct Poller {
    /// The epoll instance, which holds what each source is watched for.
    epoll: OwnedFd,
    /// The channel a `notify` writes to, whose read end the instance watches under [`WAKE`].
    wake: Pipe,
    /// The timer a wait with a timeout sets, which the instance watches under [`TIMER`].
    timer: OwnedFd,
}

impl Poller {
    /// Whether a change reaches a wait under way: it does, as epoll keeps what it watches in the
    /// kernel, and a descriptor deleted from it can close at once.
    pub(crate) const LIVE: bool = true;

    pub(crate) fn new() -> io::Result<Self> {
        let epoll = epoll::create(CreateFlags::CLOEXEC)?;
        let wake = Pipe::new()?;
        // Level-triggered, as every source is: a wake-up is reported by each wait until one
        // drains it.
        epoll::add(
            &epoll,
            wake.read_end(),
            EventData::new_u64(WAKE),
            EventFlags::IN,
        )?;
        // On the clock `Instant` reads, as the reactor's deadlines do. Level-triggered as well: a
        // timer that went off is reported until a wait reads it, or a timed wait sets it again.
        let timer = timerfd_create(
            TimerfdClockId::Monotonic,
            TimerfdFlags::CLOEXEC | TimerfdFlags::NONBLOCK,
        )?;
        epoll::add(&epoll, &timer, EventData::new_u64(TIMER), EventFlags::IN)?;

        Ok(Self { epoll, wake, timer })
    }

    /// Breaks a `wait` in progress or the next one; callable from any thread.
    pub(crate) fn notify(&self) -> io::Result<()> {
        self.wake.notify()
    }

    /// Takes the source `key` in, watched for nothing yet.
    ///
    /// epoll is told nothing: a source watched for nothing is not in the instance. The source is
    /// added as it is first watched in a direction, which is where a descriptor epoll cannot watch
    /// at all, such as a regular file, a directory or `/dev/null`, is turned away (`EPERM`).
    pub(crate) fn add(&self, _key: usize, _descriptor: RawSource) -> io::Result<()> {
        Ok(())
    }

    /// Watches the source `key` for `to` rather than for `from`, at once: a wait under way sees
    /// the change.
    ///
    /// A source that gains its first direction is added to the instance, and one that loses its
    /// last is deleted from it. Level-triggered, as the reactor's poll is: a source is reported by
    /// every wait for as long as it is ready in a direction it is watched in.
    pub(crate) fn modify(
        &self,
        key: usize,
        descriptor: RawSource,
        from: Directions,
        to: Directions,
    ) -> io::Result<()> {
        if from == to {
            return Ok(());
        }
        // SAFETY: the descriptor is the one the source `key` lent when it was registered, which
        // the source keeps open for as long as it lives, as `Source` says and as nothing done
        // through a shared reference to it can undo in safe code. The reactor calls this under the
        // lock of its map of sources, with the source in its entry there, which holds the source
        // across the call: the borrow, which lives no longer than this call, is of an open
        // descriptor throughout.
        let fd = unsafe { BorrowedFd::borrow_raw(descriptor) };
        if to.is_empty() {
            return Ok(epoll::delete(&self.epoll, fd)?);
        }

        let mut flags = EventFlags::empty();
        if to.readable {
            flags |= EventFlags::IN;
        }
        if to.writable {
            flags |= EventFlags::OUT;
        }
        let data = EventData::new_u64(key as u64);
        if from.is_empty() {
            Ok(epoll::add(&self.epoll, fd, data, flags)?)
        } else {
            Ok(epoll::modify(&self.epoll, fd, data, flags)?)
        }
    }

    /// Lets go of the source `key`, watched for `from`, at once, for a wait under way as well: its
    /// descriptor may close as soon as this returns.
    pub(crate) fn delete(
        &self,
        _key: usize,
        descriptor: RawSource,
        from: Directions,
    ) -> io::Result<()> {
        // A source watched for nothing is not in the instance.
        if from.is_empty() {
            return Ok(());
        }
        // SAFETY: the descriptor is the one the source lent when it was registered, which the
        // source keeps open for as long as it lives, as `Source` says and as nothing done through
        // a shared reference to it can undo in safe code. The reactor calls this under the lock of
        // its map of sources, as the source's registration goes, from the entry it has just taken
        // out of the map and which still holds the source, and lets go of that entry only after
        // this returns: the borrow, which lives no longer than this call, is of an open descriptor
        // throughout.
        let fd = unsafe { BorrowedFd::borrow_raw(descriptor) };

        Ok(epoll::delete(&self.epoll, fd)?)
    }

    /// Waits until a watched source is ready, `notify` is called or `timeout` passes; `None`
    /// waits without limit.
    pub(crate) fn wait(&self, timeout: Option<Duration>) -> io::Result<Vec<Ready>> {
        // On the stack, so that a wait allocates nothing for it. A wait that finds more sources
        // ready than it holds leaves the rest to the next, as `EVENTS` says.
        let mut events = [MaybeUninit::<Event>::uninit(); EVENTS];
        // A timeout other than zero is the timer's to keep, which ends the wait on time: epoll's
        // own would round it up to whole milliseconds. A zero one needs no timer, and would
        // disarm it. The timer is left set where the wait ends sooner for something else, and a
        // later wait without a timeout returns once, with nothing due, as it goes off: disarming
        // it would cost every such wait a call.
        let timeout = match timeout {
            Some(Duration::ZERO) => Some(Timespec::default()),
            Some(timeout) => {
                self.set_timer(timeout)?;

                None
            }
            None => None,
        };
        let (events, _) = match epoll::wait(&self.epoll, &mut events, timeout.as_ref()) {
            Ok(filled) => filled,
            Err(Errno::INTR) => return Ok(Vec::new()),
            Err(e) => return Err(e.into()),
        };

        let mut ready = Vec::new();
        for event in &*events {
            // Read out rather than borrowed: `Event` is packed on some platforms, and a field of a
            // packed struct cannot be borrowed.
            let (flags, data) = (event.flags, event.data);
            if data.u64() == WAKE {
                self.wake.drain();

                continue;
            }
            if data.u64() == TIMER {
                // Read, so that the timer stops being reported; it can only have gone off.
                let _ = read(&self.timer, &mut [0u8; 8]);

                continue;
            }
            // epoll reports an error or a hang-up whatever a source is watched for, and either is
            // reported in both directions, as the `poll(2)` poller reports them: whichever a
            // waiter waits in, it retries its operation and reads the outcome off that.
            let hung_up = flags.intersects(EventFlags::ERR | EventFlags::HUP);
            let directions = Directions {
                readable: flags.contains(EventFlags::IN) || hung_up,
                writable: flags.contains(EventFlags::OUT) || hung_up,
            };
            if !directions.is_empty() {
                // The key `modify` added the source under, which came from a `usize`.
                let key = data.u64() as usize;
                ready.push(Ready {
                    key,
                    directions,
                    dropped: Directions::NONE,
                });
            }
        }

        Ok(ready)
    }

    /// Sets the timer to go off once, `timeout` from now, which must not be zero: a timer set to
    /// zero is disarmed instead.
    fn set_timer(&self, timeout: Duration) -> io::Result<()> {
        // The reactor bounds every timeout to `MAX_TIMEOUT`, which a `Timespec` holds.
        let it_value = Timespec::try_from(timeout).map_err(|_| io::ErrorKind::InvalidInput)?;
        let value = Itimerspec {
            it_interval: Timespec::default(),
            it_value,
        };
        timerfd_settime(&self.timer, TimerfdTimerFlags::empty(), &value)?;

        Ok(())
    }
}

/// The token the wake pipe's read end is watched under.
///
/// A source's key is a `usize` counted up from zero that never comes near `usize::MAX`, so no
/// source takes this one or [`TIMER`], however wide a `usize` is.
const WAKE: u64 = u64::MAX;

/// The token the timer is watched under.
const TIMER: u64 = u64::MAX - 1;

/// How many ready sources one wait reports at most.
///
/// Those ready beyond this many are left for the next wait, which reports them: the poll is
/// level-triggered.
const EVENTS: usize = 256;
