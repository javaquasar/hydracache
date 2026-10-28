//! The sockets and pipes the poller watches, and what a connection does its I/O through.

use std::{
    collections::HashMap,
    io,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};

use polling::Event;
use zbus::runtime::{Interest, IoSource, traits};

use super::{Inner, Runtime, lock};

/// One source the runtime's poller watches.
pub struct RegisteredIoSource {
    inner: Arc<Inner>,
    state: Arc<SourceState>,
}

impl RegisteredIoSource {
    /// Puts `source` under the poller's watch, with a key of its own to report it by.
    pub(super) fn new(inner: &Arc<Inner>, source: IoSource) -> io::Result<Self> {
        let mut sources = lock(&inner.sources);
        let key = sources.next_key;
        let state = Arc::new(SourceState {
            source,
            wakers: Mutex::default(),
            key,
        });

        // SAFETY: this source deletes itself from the poller when it is dropped, and it holds the
        // state below, which owns a clone of the `IoSource`. The descriptor therefore stays open
        // until after that delete has run.
        unsafe { inner.poller.add(&state.source, Event::none(key)) }?;

        sources.next_key += 1;
        sources.states.insert(key, state.clone());

        Ok(Self {
            inner: inner.clone(),
            state,
        })
    }
}

impl traits::PollIo for RegisteredIoSource {
    fn poll_io<T>(
        &self,
        cx: &mut Context<'_>,
        interest: Interest,
        mut operation: impl FnMut() -> io::Result<T>,
    ) -> Poll<io::Result<T>> {
        match operation() {
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
            result => return Poll::Ready(result),
        }

        // Asking the poller while the wakers are held keeps the two in step: what the poller
        // watches for this source is exactly what someone is waiting for.
        let mut wakers = lock(&self.state.wakers);
        match interest {
            Interest::Readable => wakers.readable = Some(cx.waker().clone()),
            Interest::Writable => wakers.writable = Some(cx.waker().clone()),
            // `Interest` is open, and this runtime watches for the two readiness kinds it knows.
            _ => return Poll::Ready(Err(io::ErrorKind::Unsupported.into())),
        }
        let event = Event::new(
            self.state.key,
            wakers.readable.is_some(),
            wakers.writable.is_some(),
        );
        // One-shot interest is level-triggered, so a source that went ready between the
        // operation above and this call is reported as soon as the runtime waits again.
        if let Err(e) = self.inner.poller.modify(&self.state.source, event) {
            return Poll::Ready(Err(e));
        }

        Poll::Pending
    }
}

impl Drop for RegisteredIoSource {
    fn drop(&mut self) {
        lock(&self.inner.sources).states.remove(&self.state.key);
        let _ = self.inner.poller.delete(&self.state.source);
    }
}

impl Runtime {
    /// Wakes whoever waits on the source `key` names.
    ///
    /// Both directions are woken for either event: the poller watches every source in one-shot
    /// mode, so one event leaves the whole source disarmed, and a waiter has to run again to ask
    /// for the readiness it still wants.
    pub(super) fn wake_source(&self, key: usize) {
        let Some(state) = lock(&self.0.sources).states.get(&key).cloned() else {
            return;
        };
        let (readable, writable) = {
            let mut wakers = lock(&state.wakers);

            (wakers.readable.take(), wakers.writable.take())
        };

        for waker in [readable, writable].into_iter().flatten() {
            waker.wake();
        }
    }
}

/// The sources the poller watches, under the keys it reports them by.
#[derive(Default)]
pub(super) struct Sources {
    pub(super) states: HashMap<usize, Arc<SourceState>>,
    next_key: usize,
}

/// One watched source: the descriptor, and who to wake for each direction of it.
pub(super) struct SourceState {
    source: IoSource,
    pub(super) wakers: Mutex<Wakers>,
    key: usize,
}

/// Who waits for each direction of one source.
#[derive(Default)]
pub(super) struct Wakers {
    pub(super) readable: Option<Waker>,
    pub(super) writable: Option<Waker>,
}
