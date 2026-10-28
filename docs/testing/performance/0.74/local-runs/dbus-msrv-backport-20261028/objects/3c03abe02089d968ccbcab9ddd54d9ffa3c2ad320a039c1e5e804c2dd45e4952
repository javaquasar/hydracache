//! The wait on Apple's platforms: `select(2)` over the sources' descriptors and a pipe that breaks
//! the wait.
//!
//! Neither `poll(2)` nor kqueue can watch a terminal device there: `poll(2)` reports a descriptor
//! of `/dev/tty` or `/dev/null` as invalid, which a wait would take for ready every time, and
//! kqueue turns one away, while `select(2)` watches it as it does any other. rustix calls the
//! variant of `select(2)` that Darwin has for sets of any length, so `FD_SETSIZE` bounds neither
//! how many descriptors a wait watches nor how high they go.
//!
//! `select(2)` takes the whole set of descriptors on every call, as `poll(2)` does, so this poller
//! keeps the list it is told about and hands it over afresh on each wait.

use std::{io, os::fd::AsRawFd, time::Duration};

use rustix::{
    event::{FdSetElement, FdSetIter, Timespec, fd_set_insert, fd_set_num_elements, select},
    io::Errno,
};

use super::{Directions, RawSource, Ready, list::List, pipe::Pipe};

pub(crate) struct Poller {
    wake: Pipe,
    list: List,
}

impl Poller {
    /// Whether a change reaches a wait under way: it does not, as each wait copies the list as it
    /// starts.
    pub(crate) const LIVE: bool = false;

    pub(crate) fn new() -> io::Result<Self> {
        Ok(Self {
            wake: Pipe::new()?,
            list: List::new(),
        })
    }

    /// Breaks a `wait` in progress or the next one; callable from any thread.
    pub(crate) fn notify(&self) -> io::Result<()> {
        self.wake.notify()
    }

    /// Takes the source `key` in, watched for nothing yet.
    pub(crate) fn add(&self, _key: usize, _descriptor: RawSource) -> io::Result<()> {
        self.list.add();

        Ok(())
    }

    /// Watches the source `key` for `to` rather than for `from`, from the next wait on.
    pub(crate) fn modify(
        &self,
        key: usize,
        descriptor: RawSource,
        _from: Directions,
        to: Directions,
    ) -> io::Result<()> {
        self.list.modify(key, descriptor, to);

        Ok(())
    }

    /// Lets go of the source `key`, watched for `from`, from the next wait on.
    pub(crate) fn delete(
        &self,
        key: usize,
        _descriptor: RawSource,
        _from: Directions,
    ) -> io::Result<()> {
        self.list.delete(key);

        Ok(())
    }

    /// Waits until a watched source is ready, `notify` is called or `timeout` passes; `None`
    /// waits without limit.
    ///
    /// No except set is handed over. What Darwin reports there is out-of-band data, which nothing
    /// here reads, while a descriptor with an error on it, or whose peer hung up, is reported in
    /// the read and write sets, as a read or a write on it then returns at once with the error or
    /// the end.
    pub(crate) fn wait(&self, timeout: Option<Duration>) -> io::Result<Vec<Ready>> {
        let watched = self.list.snapshot();
        let wake = self.wake.read_end().as_raw_fd();
        // `select` looks at the descriptors below `nfds` alone, so it is one past the highest of
        // them.
        let nfds = watched
            .iter()
            .map(|&(_key, descriptor, _directions)| descriptor)
            .fold(wake, RawSource::max)
            + 1;
        // Each set is a bit for every descriptor below `nfds`, with room for the pipe's read end
        // and every watched source, the most that go in either of them.
        let len = fd_set_num_elements(watched.len() + 1, nfds);
        let mut readable = vec![FdSetElement::default(); len];
        let mut writable = vec![FdSetElement::default(); len];
        fd_set_insert(&mut readable, wake);
        for &(_key, descriptor, directions) in &watched {
            if directions.readable {
                fd_set_insert(&mut readable, descriptor);
            }
            if directions.writable {
                fd_set_insert(&mut writable, descriptor);
            }
        }
        // The reactor bounds every timeout to `MAX_TIMEOUT`, which a `Timespec` holds and `select`
        // takes. rustix hands it over rounded up to whole microseconds, so a deadline less than a
        // microsecond ahead is waited for rather than turned into a wait of no length at all.
        let timeout = timeout.and_then(|t| Timespec::try_from(t).ok());
        // SAFETY: every descriptor in the two sets is open across the call, which is what `select`
        // asks. The pipe's read end belongs to `self`, which outlives the call. Each of the others
        // is the one a source lent when it was registered, which the source keeps open for as long
        // as it lives, as `Source` says and as nothing done through a shared reference to it can
        // undo in safe code. The reactor holds the source while it is in the list, and keeps it
        // past its registration, until this wait returns, where the registration goes while this
        // wait runs. Each set holds a bit for every descriptor below `nfds`, which rustix checks
        // before the call, and the kernel reads and writes no more of either than that.
        let found = unsafe {
            select(
                nfds,
                Some(&mut readable),
                Some(&mut writable),
                None,
                timeout.as_ref(),
            )
        };
        match found {
            Ok(_) => {}
            // What the sets hold after an interrupted call is left open, so none of it is read.
            Err(Errno::INTR) => return Ok(Vec::new()),
            Err(e) => return Err(e.into()),
        }
        let readable = members(&readable);
        let writable = members(&writable);
        if readable.binary_search(&wake).is_ok() {
            self.wake.drain();
        }

        Ok(watched
            .iter()
            .filter_map(|&(key, descriptor, _directions)| {
                let directions = Directions {
                    readable: readable.binary_search(&descriptor).is_ok(),
                    writable: writable.binary_search(&descriptor).is_ok(),
                };

                (!directions.is_empty()).then_some(Ready {
                    key,
                    directions,
                    dropped: Directions::NONE,
                })
            })
            .collect())
    }
}

/// The descriptors `set` holds once `select` has left behind what it found, in ascending order.
///
/// `select` rewrites each set it is given as the descriptors of that set its answer applies to.
/// rustix offers no look at a single bit, only a walk over the set, which goes from the lowest
/// bit up but does not promise to: the sort makes sure, at next to no cost where the walk has
/// done it already.
fn members(set: &[FdSetElement]) -> Vec<RawSource> {
    let mut members: Vec<_> = FdSetIter::new(set).collect();
    members.sort_unstable();

    members
}
