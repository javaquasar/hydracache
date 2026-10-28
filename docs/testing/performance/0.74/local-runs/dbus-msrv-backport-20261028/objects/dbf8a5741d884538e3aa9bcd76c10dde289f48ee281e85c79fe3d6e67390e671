//! What a poller that takes its sources as a list on every wait keeps of what it was told.

use std::{
    collections::HashMap,
    sync::{Mutex, MutexGuard, PoisonError},
};

use super::{Directions, RawSource};

/// The sources a poller watches in at least one direction, under the keys it reports them by, and
/// how many it was handed in all.
///
/// Behind a lock of its own: the reactor tells the poller about a change from whichever thread
/// makes it, while the thread driving the runtime copies the list as each wait starts.
pub(super) struct List {
    inner: Mutex<Listed>,
}

impl List {
    pub(super) fn new() -> Self {
        Self {
            inner: Mutex::new(Listed {
                watched: HashMap::new(),
                added: 0,
            }),
        }
    }

    /// Counts a source in, watched for nothing yet; how many there are now.
    pub(super) fn add(&self) -> usize {
        let mut listed = self.lock();
        listed.added += 1;

        listed.added
    }

    /// Watches the source `key`, whose descriptor is `descriptor`, for `to`, or leaves it out of
    /// the waits from here on where `to` names neither direction.
    pub(super) fn modify(&self, key: usize, descriptor: RawSource, to: Directions) {
        let mut listed = self.lock();
        if to.is_empty() {
            listed.watched.remove(&key);
        } else {
            listed.watched.insert(key, (descriptor, to));
        }
    }

    /// Counts the source `key` out, and leaves it out of the waits from here on.
    pub(super) fn delete(&self, key: usize) {
        let mut listed = self.lock();
        listed.watched.remove(&key);
        listed.added -= 1;
    }

    /// What a wait about to start watches: each source watched in at least one direction, with
    /// its descriptor and those directions.
    pub(super) fn snapshot(&self) -> Vec<(usize, RawSource, Directions)> {
        self.lock()
            .watched
            .iter()
            .map(|(&key, &(descriptor, directions))| (key, descriptor, directions))
            .collect()
    }

    /// Takes the lock. A panic cannot leave the list half-changed, as nothing that panics is
    /// called with it held, so a poisoned lock is taken all the same.
    fn lock(&self) -> MutexGuard<'_, Listed> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// What [`List`]'s lock guards.
struct Listed {
    /// The sources watched in at least one direction: the descriptor of each, and those
    /// directions.
    watched: HashMap<usize, (RawSource, Directions)>,
    /// How many sources were added and not yet deleted, watched or not.
    added: usize,
}
