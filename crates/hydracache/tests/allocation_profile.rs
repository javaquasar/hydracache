use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use hydracache::{CacheOptions, HydraCache};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

struct CountingAllocator;

static COUNTING_ENABLED: AtomicBool = AtomicBool::new(false);
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static DEALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static REALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static ALLOCATED_BYTES: AtomicUsize = AtomicUsize::new(0);
static DEALLOCATED_BYTES: AtomicUsize = AtomicUsize::new(0);
static PROFILE_LOCK: Mutex<()> = Mutex::const_new(());

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: This allocator only observes allocation metadata and delegates
        // the actual allocation to the platform allocator unchanged.
        let pointer = unsafe { System.alloc(layout) };
        if COUNTING_ENABLED.load(Ordering::Relaxed) && !pointer.is_null() {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            ALLOCATED_BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        if COUNTING_ENABLED.load(Ordering::Relaxed) {
            DEALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            DEALLOCATED_BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        }
        // SAFETY: The pointer and layout come from the caller of GlobalAlloc and
        // are passed through to the same underlying allocator unchanged.
        unsafe { System.dealloc(pointer, layout) };
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: This delegates reallocation to the platform allocator with
        // the original pointer, layout, and requested new size unchanged.
        let new_pointer = unsafe { System.realloc(pointer, layout, new_size) };
        if COUNTING_ENABLED.load(Ordering::Relaxed) && !new_pointer.is_null() {
            REALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            ALLOCATED_BYTES.fetch_add(new_size, Ordering::Relaxed);
            DEALLOCATED_BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        }
        new_pointer
    }
}

#[derive(Debug, Clone, Copy)]
struct AllocationSnapshot {
    allocations: usize,
    deallocations: usize,
    reallocations: usize,
    allocated_bytes: usize,
    deallocated_bytes: usize,
}

impl AllocationSnapshot {
    fn current() -> Self {
        Self {
            allocations: ALLOCATIONS.load(Ordering::Relaxed),
            deallocations: DEALLOCATIONS.load(Ordering::Relaxed),
            reallocations: REALLOCATIONS.load(Ordering::Relaxed),
            allocated_bytes: ALLOCATED_BYTES.load(Ordering::Relaxed),
            deallocated_bytes: DEALLOCATED_BYTES.load(Ordering::Relaxed),
        }
    }

    fn emit(self, scenario: &str, operations: usize) {
        eprintln!(
            "allocation-profile {scenario}: operations={operations}, allocations={allocations}, reallocations={reallocations}, deallocations={deallocations}, allocated_bytes={allocated_bytes}, deallocated_bytes={deallocated_bytes}",
            allocations = self.allocations,
            reallocations = self.reallocations,
            deallocations = self.deallocations,
            allocated_bytes = self.allocated_bytes,
            deallocated_bytes = self.deallocated_bytes,
        );
    }
}

fn start_counting() {
    ALLOCATIONS.store(0, Ordering::Relaxed);
    DEALLOCATIONS.store(0, Ordering::Relaxed);
    REALLOCATIONS.store(0, Ordering::Relaxed);
    ALLOCATED_BYTES.store(0, Ordering::Relaxed);
    DEALLOCATED_BYTES.store(0, Ordering::Relaxed);
    COUNTING_ENABLED.store(true, Ordering::Relaxed);
}

fn finish_counting() -> AllocationSnapshot {
    COUNTING_ENABLED.store(false, Ordering::Relaxed);
    AllocationSnapshot::current()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct AllocationValue {
    id: u64,
    name: String,
    labels: Vec<String>,
}

impl AllocationValue {
    fn new(id: u64) -> Self {
        Self {
            id,
            name: format!("allocation-value-{id}"),
            labels: vec![format!("label-{id}"), "cached".to_owned()],
        }
    }
}

#[tokio::test]
#[ignore = "manual allocation profile; run with --ignored --nocapture"]
async fn profile_hot_get_hits() {
    let _profile_guard = PROFILE_LOCK.lock().await;
    let cache = HydraCache::local().build();
    cache
        .put(
            "allocation:hot",
            AllocationValue::new(1),
            CacheOptions::new().tags(["allocation", "hot"]),
        )
        .await
        .unwrap();

    let operations = 256;
    start_counting();
    for _ in 0..operations {
        let cached: Option<AllocationValue> = cache.get("allocation:hot").await.unwrap();
        assert_eq!(cached, Some(AllocationValue::new(1)));
    }
    finish_counting().emit("hot-get-hits", operations);
}

#[tokio::test]
#[ignore = "manual allocation profile; run with --ignored --nocapture"]
async fn profile_contains_key_metadata_hits() {
    let _profile_guard = PROFILE_LOCK.lock().await;
    let cache = HydraCache::local().build();
    cache
        .put(
            "allocation:contains",
            AllocationValue::new(2),
            CacheOptions::new().tags(["allocation", "contains"]),
        )
        .await
        .unwrap();

    let operations = 256;
    start_counting();
    for _ in 0..operations {
        assert!(cache.contains_key("allocation:contains").await);
    }
    finish_counting().emit("contains-key-hits", operations);
}

#[tokio::test]
#[ignore = "manual allocation profile; run with --ignored --nocapture"]
async fn profile_event_preflight_modes() {
    let _profile_guard = PROFILE_LOCK.lock().await;
    let operations = 128;

    let no_subscriber = HydraCache::local().build();
    no_subscriber
        .put(
            "allocation:event:no-subscriber",
            AllocationValue::new(10),
            CacheOptions::new().tags(["allocation", "events"]),
        )
        .await
        .unwrap();
    start_counting();
    for _ in 0..operations {
        let cached: Option<AllocationValue> = no_subscriber
            .get("allocation:event:no-subscriber")
            .await
            .unwrap();
        assert_eq!(cached, Some(AllocationValue::new(10)));
    }
    finish_counting().emit("event-preflight-no-subscriber", operations);

    let mutation_subscriber = HydraCache::local().build();
    let _events = mutation_subscriber.subscribe_mutations();
    mutation_subscriber
        .put(
            "allocation:event:mutation-subscriber",
            AllocationValue::new(11),
            CacheOptions::new().tags(["allocation", "events"]),
        )
        .await
        .unwrap();
    start_counting();
    for _ in 0..operations {
        let cached: Option<AllocationValue> = mutation_subscriber
            .get("allocation:event:mutation-subscriber")
            .await
            .unwrap();
        assert_eq!(cached, Some(AllocationValue::new(11)));
    }
    finish_counting().emit("event-preflight-mutation-subscriber", operations);

    let access_subscriber = HydraCache::local().enable_access_events(true).build();
    let _events = access_subscriber.subscribe_access();
    access_subscriber
        .put(
            "allocation:event:access-subscriber",
            AllocationValue::new(12),
            CacheOptions::new().tags(["allocation", "events"]),
        )
        .await
        .unwrap();
    start_counting();
    for _ in 0..operations {
        let cached: Option<AllocationValue> = access_subscriber
            .get("allocation:event:access-subscriber")
            .await
            .unwrap();
        assert_eq!(cached, Some(AllocationValue::new(12)));
    }
    finish_counting().emit("event-preflight-access-subscriber", operations);
}

#[tokio::test]
#[ignore = "manual allocation profile; run with --ignored --nocapture"]
async fn profile_typed_hot_get_hits() {
    let _profile_guard = PROFILE_LOCK.lock().await;
    let cache = HydraCache::local().build();
    let typed = cache.typed::<AllocationValue>("allocation-values");
    typed
        .put(
            "typed-hot",
            AllocationValue::new(3),
            CacheOptions::new().tags(["allocation", "typed"]),
        )
        .await
        .unwrap();

    let operations = 256;
    start_counting();
    for _ in 0..operations {
        let cached = typed.get("typed-hot").await.unwrap();
        assert_eq!(cached, Some(AllocationValue::new(3)));
    }
    finish_counting().emit("typed-hot-get-hits", operations);
}

#[tokio::test]
#[ignore = "manual allocation profile; run with --ignored --nocapture"]
async fn profile_bulk_tag_invalidation() {
    let _profile_guard = PROFILE_LOCK.lock().await;
    let cache = HydraCache::local().max_capacity(1_000_000).build();
    let entries = 256;

    start_counting();
    for id in 0..entries {
        cache
            .put(
                &format!("allocation:tenant:7:{id}"),
                AllocationValue::new(id as u64),
                CacheOptions::new().tags(["allocation", "tenant:7"]),
            )
            .await
            .unwrap();
    }
    let removed = cache.invalidate_tag("tenant:7").await.unwrap();
    let snapshot = finish_counting();

    assert_eq!(removed, entries as u64);
    snapshot.emit("bulk-tag-invalidation", entries);
}
