//! What handing a piece of blocking work to `unblock` costs, and awaiting its outcome: the work
//! itself is a value handed back at once, so that all there is to time is the way there and back.
//! `unblock` needs no runtime, so every id awaits its futures with the `block_on` of `futures`,
//! which parks the thread that awaits until the work wakes it.
//!
//! `round-trip` times one `unblock` and the `block_on` that awaits it: the cost of a lone file
//! read or host-name lookup moved off the thread of a task, beyond that of the work. It is mostly
//! that of waking two threads, each parked as it waits for the other: the thread of the pool that
//! waits for work, and then the thread that awaits the outcome. The threads of the pool outlast
//! the gap between two iterations, so the work goes to one that is waiting for it, rather than to
//! one started for it, as it would to a thread started for each piece of work.
//!
//! `burst/16` times sixteen `unblock`s handed over one after the other, and then awaited in the
//! order they were handed over: a burst of work, such as the lookups of a batch of host names, that
//! the pool runs side by side on as many of its threads as are free for it. The throughput is in
//! pieces of work.

use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use futures::executor::block_on;
use zruntime::unblock;

/// Times a round trip of one piece of work, and then of a burst of them, in one group.
fn unblock_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("unblock");

    group.throughput(Throughput::Elements(1));
    group.bench_function("round-trip", |b| {
        b.iter(|| block_on(unblock(|| black_box(1u64))));
    });

    group.throughput(Throughput::Elements(BURST));
    group.bench_function(format!("burst/{BURST}"), |b| {
        b.iter(|| {
            let works: Vec<_> = (0..BURST).map(|n| unblock(move || black_box(n))).collect();
            for (n, work) in (0..BURST).zip(works) {
                assert_eq!(block_on(work), n);
            }
        });
    });

    group.finish();
}

/// How many pieces of work the `burst` id hands over at once.
const BURST: u64 = 16;

criterion_group!(benches, unblock_benches);
criterion_main!(benches);
