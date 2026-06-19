use std::error::Error;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use hydracache_db::{
    CommitPosition, InMemoryInvalidationOutbox, InvalidationIntentBatch, InvalidationOutbox,
};
use tokio::runtime::Runtime;

#[derive(Debug)]
struct BenchError;

impl fmt::Display for BenchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("bench write failed")
    }
}

impl Error for BenchError {}

fn bench_write_without_outbox(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();
    let counter = AtomicU64::new(0);

    c.bench_function("outbox_write/write_without_outbox", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let next = counter.fetch_add(1, Ordering::Relaxed);
            simulated_write(next).await.unwrap();
        });
    });
}

fn bench_write_with_outbox(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();
    let outbox = InMemoryInvalidationOutbox::new();
    let counter = AtomicU64::new(0);
    let batch = InvalidationIntentBatch::new("bench-write")
        .invalidate_tag("users")
        .invalidate_entity("user", "42");

    c.bench_function("outbox_write/write_with_outbox", |bencher| {
        bencher.to_async(&runtime).iter(|| {
            let outbox = outbox.clone();
            let batch = batch.clone();
            let next = counter.fetch_add(1, Ordering::Relaxed);
            async move {
                simulated_write(next).await.unwrap();
                let commit = CommitPosition::new(format!("bench:{next}"));
                let inserted = outbox.enqueue("db", &commit, &batch).await.unwrap();
                black_box(inserted);
            }
        });
    });
}

async fn simulated_write(version: u64) -> Result<(), BenchError> {
    black_box(version);
    Ok(())
}

criterion_group!(
    outbox_write,
    bench_write_without_outbox,
    bench_write_with_outbox
);
criterion_main!(outbox_write);
