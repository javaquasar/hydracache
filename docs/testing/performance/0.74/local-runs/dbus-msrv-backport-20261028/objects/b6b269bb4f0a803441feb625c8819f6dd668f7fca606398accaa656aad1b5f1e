//! The rounds of messages that the `contention` benchmarks send through the channel of
//! `channel.rs`, between threads and between the tasks of a runtime alike.

use std::hint::black_box;

use zruntime::mpmc::{Receiver, Sender};

/// How many messages each of `receivers` receives in a round of `senders` senders, which is the
/// same for each, so that they receive exactly the messages that are sent and no receiver is left
/// waiting for one that never comes.
pub fn receiver_share(senders: usize, receivers: usize) -> u64 {
    let total = senders as u64 * MESSAGES;
    let share = total / receivers as u64;
    assert_eq!(
        share * receivers as u64,
        total,
        "the receivers must share the messages equally"
    );

    share
}

/// Sends `n` messages, waiting for room whenever the channel is full.
pub async fn send_n(s: &Sender<u64>, n: u64) {
    for i in 0..n {
        s.send(i).await.unwrap();
    }
}

/// Receives `n` messages, waiting whenever the channel is empty.
pub async fn recv_n(r: &Receiver<u64>, n: u64) {
    for _ in 0..n {
        black_box(r.recv().await.unwrap());
    }
}

/// How many messages each sender sends in a round.
pub const MESSAGES: u64 = 10_000;
