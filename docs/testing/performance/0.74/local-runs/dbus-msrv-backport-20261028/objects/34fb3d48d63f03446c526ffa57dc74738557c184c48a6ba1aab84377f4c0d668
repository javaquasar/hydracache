//! What a `broadcast` channel costs per message: one `Sender::broadcast` followed by a
//! `Receiver::recv` on each of 1, 2, 4 and 8 receivers, driven on a single thread with
//! `futures::executor::block_on`. The channel has a capacity of one, so every message is
//! received by all receivers before the next one is sent. `broadcast` needs no runtime.

use std::num::NonZeroUsize;

use criterion::{Criterion, criterion_group, criterion_main};
use futures::executor::block_on;
use zruntime::broadcast::channel;

/// Times sending one message and receiving it on every one of a growing set of receivers.
fn broadcast_and_recv(c: &mut Criterion) {
    let mut group = c.benchmark_group("broadcast");

    let (s, r1) = channel(NonZeroUsize::MIN);
    let mut receivers = vec![r1];
    let mut n = 0u64;

    for count in [1, 2, 4, 8] {
        while receivers.len() < count {
            let r = receivers[0].new_receiver();
            receivers.push(r);
        }
        group.bench_function(format!("1-to-{count}"), |b| {
            b.iter(|| {
                block_on(async {
                    s.broadcast(n).await.unwrap();
                    for r in receivers.iter_mut() {
                        assert_eq!(r.recv().await.unwrap(), n);
                    }
                    n += 1;
                })
            })
        });
    }

    group.finish();
}

criterion_group!(benches, broadcast_and_recv);
criterion_main!(benches);
