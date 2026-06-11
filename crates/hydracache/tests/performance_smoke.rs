use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use hydracache::{CacheEventKind, CacheOptions, HydraCache};
use serde::{Deserialize, Serialize};
use tokio::sync::Barrier;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PerfValue {
    id: u64,
    payload: String,
}

impl PerfValue {
    fn new(id: u64) -> Self {
        Self {
            id,
            payload: format!("payload-{id:04}"),
        }
    }
}

#[derive(Debug)]
struct PerfReport {
    name: &'static str,
    requests: u64,
    loader_calls: usize,
    elapsed: Duration,
}

#[derive(Debug)]
struct EventPreflightReport {
    scenario: &'static str,
    events_published: u64,
}

impl EventPreflightReport {
    fn emit(&self) {
        eprintln!(
            "event-preflight-smoke {scenario}: events_published={events_published}",
            scenario = self.scenario,
            events_published = self.events_published,
        );
    }
}

impl PerfReport {
    fn emit(&self) {
        eprintln!(
            "perf-smoke {name}: requests={requests}, loader_calls={loader_calls}, elapsed_ms={elapsed_ms}, approx_rps={rps:.0}",
            name = self.name,
            requests = self.requests,
            loader_calls = self.loader_calls,
            elapsed_ms = self.elapsed.as_millis(),
            rps = requests_per_second(self.requests, self.elapsed),
        );
    }
}

fn requests_per_second(requests: u64, elapsed: Duration) -> f64 {
    if elapsed.is_zero() {
        requests as f64
    } else {
        requests as f64 / elapsed.as_secs_f64()
    }
}

#[tokio::test]
async fn event_preflight_publishes_only_observed_event_classes() {
    let no_subscriber = HydraCache::local().build();
    no_subscriber
        .put(
            "event:no-subscriber",
            PerfValue::new(1),
            CacheOptions::new().tag("events"),
        )
        .await
        .unwrap();
    let cached: Option<PerfValue> = no_subscriber.get("event:no-subscriber").await.unwrap();
    assert_eq!(cached, Some(PerfValue::new(1)));
    assert_eq!(no_subscriber.stats().events_published, 0);
    EventPreflightReport {
        scenario: "no-subscriber",
        events_published: no_subscriber.stats().events_published,
    }
    .emit();

    let mutation_subscriber = HydraCache::local().build();
    let mut mutation_events = mutation_subscriber.subscribe_mutations();
    mutation_subscriber
        .put(
            "event:mutation",
            PerfValue::new(2),
            CacheOptions::new().tag("events"),
        )
        .await
        .unwrap();
    let event = mutation_events.recv().await.unwrap();
    assert_eq!(event.kind(), CacheEventKind::Stored);
    let cached: Option<PerfValue> = mutation_subscriber.get("event:mutation").await.unwrap();
    assert_eq!(cached, Some(PerfValue::new(2)));
    assert_eq!(mutation_subscriber.stats().events_published, 1);
    EventPreflightReport {
        scenario: "mutation-subscriber",
        events_published: mutation_subscriber.stats().events_published,
    }
    .emit();

    let disabled_access = HydraCache::local().build();
    let _access_events = disabled_access.subscribe_access();
    let cached: Option<PerfValue> = disabled_access.get("event:missing").await.unwrap();
    assert_eq!(cached, None);
    assert_eq!(disabled_access.stats().events_published, 0);
    EventPreflightReport {
        scenario: "access-subscriber-disabled",
        events_published: disabled_access.stats().events_published,
    }
    .emit();

    let enabled_access = HydraCache::local().enable_access_events(true).build();
    let mut access_events = enabled_access.subscribe_access();
    let cached: Option<PerfValue> = enabled_access.get("event:missing").await.unwrap();
    assert_eq!(cached, None);
    let event = access_events.recv().await.unwrap();
    assert_eq!(event.kind(), CacheEventKind::Miss);
    assert_eq!(enabled_access.stats().events_published, 1);
    EventPreflightReport {
        scenario: "access-subscriber-enabled",
        events_published: enabled_access.stats().events_published,
    }
    .emit();
}

#[tokio::test]
async fn hot_hit_path_avoids_loader_and_single_flight_after_warmup() {
    let cache = HydraCache::local().build();
    let loader_calls = Arc::new(AtomicUsize::new(0));

    let warmup_calls = loader_calls.clone();
    let warmup = cache
        .get_or_insert_with("perf:hot", CacheOptions::new(), move || async move {
            warmup_calls.fetch_add(1, Ordering::SeqCst);
            PerfValue::new(42)
        })
        .await
        .unwrap();
    assert_eq!(warmup, PerfValue::new(42));

    let requests = 1_024_u64;
    let started = Instant::now();
    for _ in 0..requests {
        let hit_calls = loader_calls.clone();
        let value = cache
            .get_or_insert_with("perf:hot", CacheOptions::new(), move || async move {
                hit_calls.fetch_add(1, Ordering::SeqCst);
                PerfValue::new(999)
            })
            .await
            .unwrap();
        assert_eq!(value, PerfValue::new(42));
    }
    let elapsed = started.elapsed();

    let stats = cache.stats();
    assert_eq!(loader_calls.load(Ordering::SeqCst), 1);
    assert_eq!(stats.loads, 1);
    assert_eq!(stats.single_flight_joins, 0);
    assert_eq!(stats.hits, requests);
    assert_eq!(stats.misses, 1);
    assert!(stats.hit_ratio().unwrap() > 0.99);

    PerfReport {
        name: "hot-hit-path",
        requests,
        loader_calls: loader_calls.load(Ordering::SeqCst),
        elapsed,
    }
    .emit();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn contended_hot_key_workload_uses_one_loader_and_many_hits() {
    let cache = HydraCache::local().build();
    let loader_calls = Arc::new(AtomicUsize::new(0));
    let workers = 32_u64;
    let iterations = 16_u64;
    let barrier = Arc::new(Barrier::new(workers as usize));
    let started = Instant::now();
    let mut tasks = Vec::new();

    for _ in 0..workers {
        let cache = cache.clone();
        let loader_calls = loader_calls.clone();
        let barrier = barrier.clone();
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            for _ in 0..iterations {
                let calls = loader_calls.clone();
                let value = cache
                    .get_or_insert_with(
                        "perf:contended",
                        CacheOptions::new().tag("perf"),
                        move || async move {
                            calls.fetch_add(1, Ordering::SeqCst);
                            tokio::time::sleep(Duration::from_millis(10)).await;
                            PerfValue::new(7)
                        },
                    )
                    .await
                    .unwrap();
                assert_eq!(value, PerfValue::new(7));
            }
        }));
    }

    for task in tasks {
        task.await.unwrap();
    }
    let elapsed = started.elapsed();

    let requests = workers * iterations;
    let stats = cache.stats();
    assert_eq!(loader_calls.load(Ordering::SeqCst), 1);
    assert_eq!(stats.loads, 1);
    assert!(stats.single_flight_joins >= workers - 1);
    assert!(stats.hits >= workers * (iterations - 1));
    assert!(stats.hit_ratio().unwrap() > 0.90);

    PerfReport {
        name: "contended-hot-key",
        requests,
        loader_calls: loader_calls.load(Ordering::SeqCst),
        elapsed,
    }
    .emit();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn repeated_unique_key_workload_bounds_loader_calls_by_unique_keys() {
    let cache = HydraCache::local().build();
    let loader_calls = Arc::new(AtomicUsize::new(0));
    let workers = 16_u64;
    let iterations = 32_u64;
    let unique_keys = 16_u64;
    let barrier = Arc::new(Barrier::new(workers as usize));

    for key_id in 0..unique_keys {
        let calls = loader_calls.clone();
        let value = cache
            .get_or_insert_with(
                &format!("perf:unique:{key_id}"),
                CacheOptions::new().tags(["perf", "perf:unique"]),
                move || async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    PerfValue::new(key_id)
                },
            )
            .await
            .unwrap();
        assert_eq!(value, PerfValue::new(key_id));
    }

    let started = Instant::now();
    let mut tasks = Vec::new();

    for worker in 0..workers {
        let cache = cache.clone();
        let loader_calls = loader_calls.clone();
        let barrier = barrier.clone();
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            for iteration in 0..iterations {
                let key_id = (worker + iteration) % unique_keys;
                let key = format!("perf:unique:{key_id}");
                let calls = loader_calls.clone();
                let value = cache
                    .get_or_insert_with(
                        &key,
                        CacheOptions::new().tags(["perf", "perf:unique"]),
                        move || async move {
                            calls.fetch_add(1, Ordering::SeqCst);
                            PerfValue::new(999)
                        },
                    )
                    .await
                    .unwrap();
                assert_eq!(value, PerfValue::new(key_id));
            }
        }));
    }

    for task in tasks {
        task.await.unwrap();
    }
    let elapsed = started.elapsed();

    let requests = workers * iterations;
    let stats = cache.stats();
    assert_eq!(loader_calls.load(Ordering::SeqCst), unique_keys as usize);
    assert_eq!(stats.loads, unique_keys);
    assert_eq!(stats.total_requests(), requests + unique_keys);
    assert!(stats.hits >= requests);
    assert!(stats.hit_ratio().unwrap() > 0.95);

    PerfReport {
        name: "unique-key-workload",
        requests,
        loader_calls: loader_calls.load(Ordering::SeqCst),
        elapsed,
    }
    .emit();
}

#[tokio::test]
async fn bulk_tag_invalidation_removes_large_tagged_set_without_stranding_entries() {
    let cache = HydraCache::local().build();
    let entries = 512_u64;
    let started = Instant::now();

    for id in 0..entries {
        cache
            .put(
                &format!("perf:tenant:42:{id}"),
                PerfValue::new(id),
                CacheOptions::new().tags(["perf", "tenant:42"]),
            )
            .await
            .unwrap();
    }

    let removed = cache.invalidate_tag("tenant:42").await.unwrap();
    let elapsed = started.elapsed();

    assert_eq!(removed, entries);
    assert_eq!(cache.stats().invalidations, entries);

    for id in [0, entries / 2, entries - 1] {
        let cached: Option<PerfValue> = cache.get(&format!("perf:tenant:42:{id}")).await.unwrap();
        assert_eq!(cached, None);
    }

    PerfReport {
        name: "bulk-tag-invalidation",
        requests: entries,
        loader_calls: 0,
        elapsed,
    }
    .emit();
}
