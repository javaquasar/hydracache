//! The wait on the BSDs: kqueue, over the sources' descriptors and a pipe that breaks the wait.
//!
//! The queue keeps what it watches in the kernel and is told of each change as it is made, so a
//! wait hands it nothing and costs what the sources it finds ready cost. It watches each direction
//! of a descriptor as a filter of its own, `EVFILT_READ` and `EVFILT_WRITE`, which a source gains
//! as somebody first waits in that direction and loses once nobody does, or once the kernel drops
//! it, which a wait reports. Both are level-triggered: a filter whose direction is ready is
//! reported by every wait until it is not. The read end of a pipe is watched beside them, which no
//! source can lend while the poller owns it, and a `notify` writes to the other end to break the
//! wait.
//!
//! This is not the poller of Apple's platforms, although they have kqueue as well: theirs turns a
//! terminal device away (`EINVAL`), which is why those wait with `select(2)`. A build there with
//! `--cfg zruntime_kqueue` waits on this one all the same, which is how CI runs the tests on it.

use std::{
    io,
    mem::MaybeUninit,
    os::fd::{AsRawFd, OwnedFd},
    ptr,
    time::Duration,
};

use rustix::{
    event::kqueue::{Event, EventFilter, EventFlags, kevent, kqueue},
    io::{Errno, FdFlags, fcntl_setfd},
};

use super::{Directions, RawSource, Ready, pipe::Pipe};

pub(crate) struct Poller {
    /// The queue, declared ahead of the pipe so that it closes first: the pipe's read end, which
    /// it watches, stays open for as long as the queue lives.
    queue: OwnedFd,
    /// The pipe a `notify` writes to, whose read end the queue watches.
    wake: Pipe,
}

impl Poller {
    /// Whether a change reaches a wait under way: it does, as the queue keeps what it watches in
    /// the kernel, and a descriptor deleted from it can close at once.
    pub(crate) const LIVE: bool = true;

    pub(crate) fn new() -> io::Result<Self> {
        let queue = kqueue()?;
        // rustix makes the queue with a plain `kqueue()`, which leaves close-on-exec unset.
        fcntl_setfd(&queue, FdFlags::CLOEXEC)?;
        let poller = Self {
            queue,
            wake: Pipe::new()?,
        };
        let read_end = poller.wake.read_end().as_raw_fd();
        poller.apply([change(EventFilter::Read(read_end), true, WAKE)])?;

        Ok(poller)
    }

    /// Breaks a `wait` in progress or the next one; callable from any thread.
    pub(crate) fn notify(&self) -> io::Result<()> {
        self.wake.notify()
    }

    /// Takes the source `key` in, watched for nothing yet.
    ///
    /// The queue is told nothing: it watches each direction of a descriptor as a filter of its
    /// own, which `modify` adds as that direction is first wanted, and a descriptor watched for
    /// nothing is one it has no filter of.
    pub(crate) fn add(&self, _key: usize, _descriptor: RawSource) -> io::Result<()> {
        Ok(())
    }

    /// Watches the source `key` for `to` rather than for `from`, at once, a wait under way
    /// included.
    ///
    /// Each direction gained is a filter added to the queue, reported under `key`, and each one
    /// lost is a filter deleted from it, all in one call. A filter the kernel dropped by itself,
    /// as `wait` reports, may be deleted before the reactor hears of that, on a thread other than
    /// the waiting one: such a delete finds nothing (`ENOENT`), which is not an error.
    ///
    /// A change that fails leaves its direction as it was, and the other change of the call, if
    /// any, stands. The reactor gains no more than one direction a call, so a gain that fails
    /// leaves the source watched in `from`.
    pub(crate) fn modify(
        &self,
        key: usize,
        descriptor: RawSource,
        from: Directions,
        to: Directions,
    ) -> io::Result<()> {
        let read = (from.readable != to.readable)
            .then(|| change(EventFilter::Read(descriptor), to.readable, key));
        let write = (from.writable != to.writable)
            .then(|| change(EventFilter::Write(descriptor), to.writable, key));
        match (read, write) {
            (Some(read), Some(write)) => self.apply([read, write]),
            (Some(only), None) | (None, Some(only)) => self.apply([only]),
            (None, None) => Ok(()),
        }
    }

    /// Lets go of the source `key`, watched for `from`, at once: the queue drops each filter of
    /// it there and then, so its descriptor can close as soon as this returns.
    pub(crate) fn delete(
        &self,
        key: usize,
        descriptor: RawSource,
        from: Directions,
    ) -> io::Result<()> {
        // A source watched for nothing has no filter in the queue to delete.
        if from.is_empty() {
            return Ok(());
        }

        self.modify(key, descriptor, from, Directions::NONE)
    }

    /// Waits until a watched source is ready, `notify` is called or `timeout` passes; `None`
    /// waits without limit.
    ///
    /// A source found ready in both directions comes back twice, once for each filter. One whose
    /// filter reports the end of its descriptor (`EV_EOF`) comes back as ready like any other:
    /// its reader or writer retries its operation, and sees the end or the error for itself.
    ///
    /// A filter the kernel drops as it reports it comes back as dropped as well, so that the
    /// next wait in its direction adds it again. FreeBSD does that to the write filter of a pipe,
    /// which it hangs on the pipe's other end: once that end closes, the filter is reported one
    /// last time, with `EV_EOF` and `EV_ONESHOT`, and is gone. Adding it again fails (`EPIPE`),
    /// which the next wait in that direction ends with, rather than wait on a filter that is no
    /// longer there.
    pub(crate) fn wait(&self, timeout: Option<Duration>) -> io::Result<Vec<Ready>> {
        let mut events = [MaybeUninit::uninit(); EVENTS];
        // The reactor bounds every timeout to `MAX_TIMEOUT`, which a `Timespec` holds, so rustix
        // never has to turn one it cannot convert into a wait without limit.
        //
        // SAFETY: rustix asks that every descriptor a change names stay valid for as long as the
        // queue lives. This call makes no change, and so names no descriptor; what the queue
        // watches already, it was told of by `apply`, which argues the same of each descriptor
        // it names. The events come back as plain values, and the udata of each is read for its
        // address alone, as a key.
        let events = match unsafe { kevent(&self.queue, &[], &mut events, timeout) } {
            Ok((events, _)) => events,
            Err(Errno::INTR) => return Ok(Vec::new()),
            Err(e) => return Err(e.into()),
        };

        let wake = self.wake.read_end().as_raw_fd();
        let mut ready = Vec::with_capacity(events.len());
        for event in events.iter() {
            let directions = match event.filter() {
                // Told apart by its descriptor, which no source can lend while the poller owns it,
                // rather than by its key, which a source's key may come to equal on a target
                // whose `usize` is narrow.
                EventFilter::Read(descriptor) if descriptor == wake => {
                    self.wake.drain();
                    continue;
                }
                EventFilter::Read(_) => Directions {
                    readable: true,
                    writable: false,
                },
                EventFilter::Write(_) => Directions {
                    readable: false,
                    writable: true,
                },
                // No other filter is ever added under a source's key.
                _ => continue,
            };
            let key = event.udata().addr();
            // `change` never asks for `EV_ONESHOT`, so a filter reported with it is one the
            // kernel made one-shot, and has dropped as it reported it.
            let dropped = if event.flags().contains(EventFlags::ONESHOT) {
                directions
            } else {
                Directions::NONE
            };
            ready.push(Ready {
                key,
                directions,
                dropped,
            });
        }

        Ok(ready)
    }

    /// Makes `changes` to the queue in one call, and reads back how each of them went.
    ///
    /// Each change asks for a receipt (`EV_RECEIPT`), which the queue answers with an event in
    /// the place the change has in the list: `EV_ERROR` set, and the error number of the change
    /// in its data, zero where it was made. A call that hands back receipts reports no readiness
    /// beside them, and this one has room for one receipt a change and a timeout of zero, so it
    /// never waits.
    fn apply<const N: usize>(&self, changes: [Event; N]) -> io::Result<()> {
        let mut receipts = [MaybeUninit::uninit(); N];
        // SAFETY: rustix asks that every descriptor a change names stay valid for as long as the
        // queue lives. A change names either the read end of the wake pipe, which the poller
        // owns and closes after the queue, or the descriptor a source lent when it was
        // registered. The reactor tells the poller of a change to a source only while it holds
        // that source, which keeps its descriptor open across the call; and it has every filter
        // of the source deleted, through `delete`, before it lets go of the source, so before the
        // descriptor can close. The queue drops a deleted filter there and then, and a filter it
        // still has when its descriptor closes, as one whose delete failed would be, goes with
        // that close, which deletes every filter of the descriptor: the queue never refers to a
        // closed descriptor. The udata of each change is a key carried in a pointer's place,
        // which neither the kernel nor this module dereferences.
        let (receipts, _) =
            unsafe { kevent(&self.queue, &changes, &mut receipts, Some(Duration::ZERO)) }?;
        debug_assert_eq!(receipts.len(), N, "every change comes back as a receipt");
        for (change, receipt) in changes.iter().zip(receipts.iter()) {
            // The data of an event is an error number only where `EV_ERROR` is set, as it is on
            // every receipt.
            if !receipt.flags().contains(EventFlags::ERROR) {
                continue;
            }
            let errno = match receipt.data() {
                0 => continue,
                // The kernel writes an `int` there, which an `i32` holds whole.
                errno => Errno::from_raw_os_error(errno as i32),
            };
            let gone = errno == Errno::NOENT && change.flags().contains(EventFlags::DELETE);
            if !gone {
                return Err(errno.into());
            }
        }

        Ok(())
    }
}

/// The change that has the queue watch `filter`, reported under `key`, where `watched`, and that
/// deletes it from the queue otherwise, with a receipt asked for either way.
///
/// Neither `EV_CLEAR` nor `EV_ONESHOT` is set, so a filter added is level-triggered: every wait
/// reports it for as long as its direction is ready, which is what the reactor expects of a
/// poller.
fn change(filter: EventFilter, watched: bool, key: usize) -> Event {
    let action = if watched {
        EventFlags::ADD
    } else {
        EventFlags::DELETE
    };
    // The key is a bare address, with no provenance, as nothing is ever reached through it.
    let udata = ptr::without_provenance_mut(key);

    Event::new(filter, action | EventFlags::RECEIPT, udata)
}

/// The key the wake pipe's read end is added under, which nothing reads: a wait tells the pipe
/// apart by its descriptor.
const WAKE: usize = usize::MAX;

/// The most events one wait reads.
///
/// Those past it stay ready in the queue, whose filters are level-triggered, and the next wait
/// reports them.
const EVENTS: usize = 256;
