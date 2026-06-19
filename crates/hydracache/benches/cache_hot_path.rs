use std::error::Error;
use std::fmt;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use hydracache::{CacheOptions, HydraCache};
use tokio::runtime::Runtime;

#[derive(Debug)]
struct BenchError;

impl fmt::Display for BenchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("bench loader failed")
    }
}

impl Error for BenchError {}

fn bench_hit(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();
    let cache = HydraCache::local().build();
    runtime
        .block_on(cache.put("hit", 42_u64, CacheOptions::new()))
        .unwrap();

    c.bench_function("hot_path/hit", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let value = cache.get::<u64>(black_box("hit")).await.unwrap();
            black_box(value);
        });
    });
}

fn bench_miss(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();
    let cache = HydraCache::local().build();

    c.bench_function("hot_path/miss", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let value = cache.get::<u64>(black_box("missing")).await.unwrap();
            black_box(value);
        });
    });
}

fn bench_single_flight(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();

    c.bench_function("hot_path/single_flight_16", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let cache = HydraCache::local().build();
            let loads = Arc::new(AtomicUsize::new(0));
            let mut tasks = Vec::with_capacity(16);

            for _ in 0..16 {
                let cache = cache.clone();
                let loads = loads.clone();
                tasks.push(tokio::spawn(async move {
                    cache
                        .get_or_load("single-flight", CacheOptions::new(), move || async move {
                            loads.fetch_add(1, Ordering::Relaxed);
                            Ok::<_, BenchError>(42_u64)
                        })
                        .await
                        .unwrap()
                }));
            }

            for task in tasks {
                black_box(task.await.unwrap());
            }
            black_box(loads.load(Ordering::Relaxed));
        });
    });
}

fn bench_event_publish(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();
    let no_subscriber = HydraCache::local().build();
    let with_subscriber = HydraCache::local().event_buffer_capacity(1_000_000).build();
    let _events = with_subscriber.subscribe_mutations();
    let counter = Arc::new(AtomicU64::new(0));

    c.bench_function("hot_path/event_publish_no_subscriber", |bencher| {
        bencher.to_async(&runtime).iter(|| {
            let cache = no_subscriber.clone();
            let counter = counter.clone();
            async move {
                let next = counter.fetch_add(1, Ordering::Relaxed);
                let key = format!("event:{next}");
                cache.put(&key, 1_u64, CacheOptions::new()).await.unwrap();
            }
        });
    });

    c.bench_function("hot_path/event_publish_with_subscriber", |bencher| {
        bencher.to_async(&runtime).iter(|| {
            let cache = with_subscriber.clone();
            let counter = counter.clone();
            async move {
                let next = counter.fetch_add(1, Ordering::Relaxed);
                let key = format!("observed-event:{next}");
                cache.put(&key, 1_u64, CacheOptions::new()).await.unwrap();
            }
        });
    });
}

criterion_group!(
    hot_path,
    bench_hit,
    bench_miss,
    bench_single_flight,
    bench_event_publish
);
criterion_main!(hot_path);
