//! What an `mpmc` channel costs on a single thread, every id a different shape of use of one
//! channel of `u64`s. They are the single-thread rows of the table in the "Performance" section of
//! the `mpmc` module's documentation, which times the channel against async-channel and tokio's
//! `mpsc`, kept here as regression checks of this channel alone: the comparison needs those two
//! crates, which this one does not depend on. The rows of threads, and the ids that run the same
//! rounds as tasks on a runtime, are timed in `contention.rs`, as only the clock can measure them.
//!
//! `mpmc` needs no runtime, so the ids drive their futures with the `block_on` of `futures`, not
//! `zruntime::block_on`. Every id sends one message and receives it before it is timed, so that
//! what the channel allocates lazily on its first use is allocated by then.
//!
//! - `ping-bounded1` times one `send(..).await` and one `recv().await` on a channel with room for
//!   one message, with one `block_on` around the pair: the uncontended path, and so the cost of a
//!   channel whose receivers keep up with its senders. The message is received before the next is
//!   sent, so no send finds the channel full, no receive finds it empty, and nothing waits. Every
//!   notification reaches nobody: the two that a send makes to the receivers waiting for a message,
//!   and the one that a receive makes to the senders waiting for room. Each finds that nobody
//!   listens with one atomic look at its event, and takes no lock.
//! - `ping-unbounded` times the same pair on an unbounded channel. It differs from `ping-bounded1`
//!   in one notification: an unbounded channel has no sender waiting for room, so a receive from it
//!   makes none, where a receive from a bounded one looks for a sender to tell.
//! - `burst-bounded1024` and `burst-unbounded` time 1024 `try_send`s and then as many `try_recv`s,
//!   with no `block_on` and no future: the channel's queue and the bookkeeping around it, with
//!   nothing to wait for. The bounded channel has room for all 1024. The throughput is in messages.

use std::hint::black_box;

use criterion::{
    BenchmarkGroup, Criterion, Throughput, criterion_group, criterion_main, measurement::WallTime,
};
use futures::executor::block_on;

#[path = "common/channel.rs"]
mod channel;

/// Times every id, one after the other, in one group.
fn channel_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("mpmc");

    ping(&mut group, "ping-bounded1", Some(1));
    ping(&mut group, "ping-unbounded", None);
    burst(&mut group, "burst-bounded1024", Some(BURST as usize));
    burst(&mut group, "burst-unbounded", None);

    group.finish();
}

/// Times sending one message and receiving it, on a channel of capacity `cap` (unbounded if
/// `None`), all on a single thread.
fn ping(group: &mut BenchmarkGroup<'_, WallTime>, id: &str, cap: Option<usize>) {
    group.throughput(Throughput::Elements(1));

    let (s, r) = channel::new(cap);
    let mut n = 0u64;
    group.bench_function(id, |b| {
        b.iter(|| {
            block_on(async {
                s.send(n).await.unwrap();
                assert_eq!(r.recv().await.unwrap(), n);
            });
            n += 1;
        })
    });
}

/// Times sending [`BURST`] messages with `try_send` and then receiving them with `try_recv`, on a
/// channel of capacity `cap` (unbounded if `None`), all on a single thread.
fn burst(group: &mut BenchmarkGroup<'_, WallTime>, id: &str, cap: Option<usize>) {
    group.throughput(Throughput::Elements(BURST));

    let (s, r) = channel::new(cap);
    group.bench_function(id, |b| {
        b.iter(|| {
            for i in 0..BURST {
                s.try_send(i).unwrap();
            }
            for _ in 0..BURST {
                black_box(r.try_recv().unwrap());
            }
        })
    });
}

/// How many messages the `burst-*` ids send, and then receive.
const BURST: u64 = 1024;

criterion_group!(benches, channel_benches);
criterion_main!(benches);
