//! What spawning a task costs: a hundred tasks spawned on the runtime and awaited, inside one
//! `zruntime::block_on`, the shape of a program that does all of its work inside one call. The
//! runtime handle the tasks are spawned on is taken once, in a `block_on` of its own, before the
//! group starts, as in `runtime.rs`. Measured by the clock, as that file's ids are: the tasks wait
//! on nothing, but the runtime waits on its reactor at every turn of `block_on`, and the helper
//! thread that holds the runtime's seat between two `block_on` calls parks and unparks around
//! each, all in the kernel, which CodSpeed's CPU simulation leaves out.

use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use executor::{ZruntimeExecutor, runtime_handle};

#[path = "common/executor.rs"]
mod executor;

fn spawn_benches(c: &mut Criterion) {
    let runtime = runtime_handle();

    let mut group = c.benchmark_group("spawn");
    group.throughput(Throughput::Elements(100));
    group.bench_function("100-tasks", |b| {
        b.to_async(ZruntimeExecutor).iter(|| async {
            let tasks: Vec<_> = (0..100u32)
                .map(|i| runtime.spawn("bench", async move { i }))
                .collect();
            for task in tasks {
                black_box(task.await.unwrap());
            }
        });
    });
    group.finish();
}

criterion_group!(benches, spawn_benches);
criterion_main!(benches);
