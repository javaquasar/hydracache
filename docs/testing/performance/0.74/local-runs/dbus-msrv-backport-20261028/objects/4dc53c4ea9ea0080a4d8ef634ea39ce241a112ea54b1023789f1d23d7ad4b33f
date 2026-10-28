//! The `mpmc` channel of `u64`s that the `mpmc` and `contention` benchmarks time. The rounds of
//! messages that `contention` sends through it are in `rounds.rs`.

use std::num::NonZeroUsize;

use futures::executor::block_on;
use zruntime::mpmc::{Receiver, Sender, bounded, unbounded};

/// A channel with room for `cap` messages, or an unbounded one if `cap` is `None`.
///
/// One message is sent through it and received before it is handed back, so that whatever the
/// channel allocates lazily on its first use is allocated before any measurement starts.
pub fn new(cap: Option<usize>) -> (Sender<u64>, Receiver<u64>) {
    let (s, r) = match cap {
        Some(cap) => bounded(NonZeroUsize::new(cap).unwrap()),
        None => unbounded(),
    };
    block_on(async {
        s.send(0).await.unwrap();
        r.recv().await.unwrap();
    });

    (s, r)
}
