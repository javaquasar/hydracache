use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::spin_loop;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use hydracache::{CacheOptions, HydraCache};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

struct CountingAllocator;

const TRACKED_ALLOCATION_SLOTS: usize = 32_768;
const TRACKING_TOMBSTONE: usize = 1;

struct TrackedAllocation {
    pointer: AtomicUsize,
    epoch: AtomicU64,
    bytes: AtomicUsize,
}

impl TrackedAllocation {
    const fn empty() -> Self {
        Self {
            pointer: AtomicUsize::new(0),
            epoch: AtomicU64::new(0),
            bytes: AtomicUsize::new(0),
        }
    }
}

static ACTIVE_EPOCH: AtomicU64 = AtomicU64::new(0);
static NEXT_EPOCH: AtomicU64 = AtomicU64::new(1);
static COUNTING_IN_FLIGHT: AtomicUsize = AtomicUsize::new(0);
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static ZEROED_ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static DEALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static REALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static ALLOCATED_BYTES: AtomicUsize = AtomicUsize::new(0);
static DEALLOCATED_BYTES: AtomicUsize = AtomicUsize::new(0);
static LIVE_BYTES: AtomicUsize = AtomicUsize::new(0);
static PEAK_LIVE_BYTES: AtomicUsize = AtomicUsize::new(0);
static TRACKING_OVERFLOWS: AtomicUsize = AtomicUsize::new(0);
static TRACKED_ALLOCATIONS: [TrackedAllocation; TRACKED_ALLOCATION_SLOTS] =
    [const { TrackedAllocation::empty() }; TRACKED_ALLOCATION_SLOTS];
static PROFILE_LOCK: Mutex<()> = Mutex::const_new(());

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: This allocator only observes allocation metadata and delegates
        // the actual allocation to the platform allocator unchanged.
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            record_allocation(pointer, layout.size(), false);
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: The request is delegated unchanged to the platform allocator.
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            record_allocation(pointer, layout.size(), true);
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        let active_epoch = enter_active_epoch();
        let tracked = untrack_allocation(pointer);
        if active_epoch != 0 {
            DEALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            DEALLOCATED_BYTES.fetch_add(layout.size(), Ordering::Relaxed);
            if tracked.is_some_and(|(epoch, _)| epoch == active_epoch) {
                subtract_live_bytes(layout.size());
            }
        }
        leave_active_epoch(active_epoch);
        // SAFETY: The pointer and layout come from the caller of GlobalAlloc and
        // are passed through to the same underlying allocator unchanged.
        unsafe { System.dealloc(pointer, layout) };
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let active_epoch = enter_active_epoch();
        let tracked = untrack_allocation(pointer);
        if active_epoch != 0 && tracked.is_some_and(|(epoch, _)| epoch == active_epoch) {
            subtract_live_bytes(layout.size());
        }
        // SAFETY: This delegates reallocation to the platform allocator with
        // the original pointer, layout, and requested new size unchanged.
        let new_pointer = unsafe { System.realloc(pointer, layout, new_size) };
        if new_pointer.is_null() {
            if let Some((epoch, bytes)) = tracked {
                if track_allocation(pointer, epoch, bytes) && epoch == active_epoch {
                    add_live_bytes(bytes);
                }
            }
        } else if active_epoch != 0 {
            REALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            ALLOCATED_BYTES.fetch_add(new_size, Ordering::Relaxed);
            DEALLOCATED_BYTES.fetch_add(layout.size(), Ordering::Relaxed);
            if track_allocation(new_pointer, active_epoch, new_size) {
                add_live_bytes(new_size);
            }
        } else if let Some((epoch, _)) = tracked {
            let _ = track_allocation(new_pointer, epoch, new_size);
        }
        leave_active_epoch(active_epoch);
        new_pointer
    }
}

fn record_allocation(pointer: *mut u8, bytes: usize, zeroed: bool) {
    let epoch = enter_active_epoch();
    if epoch == 0 {
        return;
    }
    ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
    if zeroed {
        ZEROED_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
    }
    ALLOCATED_BYTES.fetch_add(bytes, Ordering::Relaxed);
    if track_allocation(pointer, epoch, bytes) {
        add_live_bytes(bytes);
    }
    leave_active_epoch(epoch);
}

fn enter_active_epoch() -> u64 {
    let epoch = ACTIVE_EPOCH.load(Ordering::Acquire);
    if epoch == 0 {
        return 0;
    }
    COUNTING_IN_FLIGHT.fetch_add(1, Ordering::AcqRel);
    if ACTIVE_EPOCH.load(Ordering::Acquire) == epoch {
        epoch
    } else {
        COUNTING_IN_FLIGHT.fetch_sub(1, Ordering::Release);
        0
    }
}

fn leave_active_epoch(epoch: u64) {
    if epoch != 0 {
        COUNTING_IN_FLIGHT.fetch_sub(1, Ordering::Release);
    }
}

fn pointer_slot(pointer: *mut u8) -> usize {
    (pointer as usize >> 3) % TRACKED_ALLOCATION_SLOTS
}

fn track_allocation(pointer: *mut u8, epoch: u64, bytes: usize) -> bool {
    let pointer_value = pointer as usize;
    let start = pointer_slot(pointer);
    for offset in 0..TRACKED_ALLOCATION_SLOTS {
        let slot = &TRACKED_ALLOCATIONS[(start + offset) % TRACKED_ALLOCATION_SLOTS];
        let candidate = slot.pointer.load(Ordering::Acquire);
        if (candidate == 0 || candidate == TRACKING_TOMBSTONE)
            && slot
                .pointer
                .compare_exchange(
                    candidate,
                    pointer_value,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_ok()
        {
            slot.bytes.store(bytes, Ordering::Relaxed);
            slot.epoch.store(epoch, Ordering::Release);
            return true;
        }
    }
    TRACKING_OVERFLOWS.fetch_add(1, Ordering::Relaxed);
    false
}

fn untrack_allocation(pointer: *mut u8) -> Option<(u64, usize)> {
    let pointer_value = pointer as usize;
    let start = pointer_slot(pointer);
    for offset in 0..TRACKED_ALLOCATION_SLOTS {
        let slot = &TRACKED_ALLOCATIONS[(start + offset) % TRACKED_ALLOCATION_SLOTS];
        let candidate = slot.pointer.load(Ordering::Acquire);
        if candidate == 0 {
            return None;
        }
        if candidate == pointer_value {
            let epoch = slot.epoch.load(Ordering::Acquire);
            let bytes = slot.bytes.load(Ordering::Relaxed);
            slot.epoch.store(0, Ordering::Relaxed);
            slot.bytes.store(0, Ordering::Relaxed);
            slot.pointer.store(TRACKING_TOMBSTONE, Ordering::Release);
            return Some((epoch, bytes));
        }
    }
    None
}

fn add_live_bytes(bytes: usize) {
    let previous = LIVE_BYTES
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            Some(current.saturating_add(bytes))
        })
        .unwrap_or_else(|current| current);
    let live = previous.saturating_add(bytes);
    let _ = PEAK_LIVE_BYTES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |peak| {
        (live > peak).then_some(live)
    });
}

fn subtract_live_bytes(bytes: usize) {
    let _ = LIVE_BYTES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
        Some(current.saturating_sub(bytes))
    });
}

#[derive(Debug, Clone, Copy)]
struct AllocationSnapshot {
    allocations: usize,
    zeroed_allocations: usize,
    deallocations: usize,
    reallocations: usize,
    allocated_bytes: usize,
    deallocated_bytes: usize,
    live_bytes: usize,
    peak_live_bytes: usize,
    tracking_overflows: usize,
}

const GEOMETRIC_CARDINALITIES: [usize; 5] = [1, 10, 100, 1_000, 10_000];

fn has_positive_retained_slope(samples: &[(usize, usize)]) -> bool {
    samples.len() >= 3
        && samples
            .windows(2)
            .all(|window| window[1].0 > window[0].0 && window[1].1 > window[0].1)
}

impl AllocationSnapshot {
    fn current() -> Self {
        Self {
            allocations: ALLOCATIONS.load(Ordering::Relaxed),
            zeroed_allocations: ZEROED_ALLOCATIONS.load(Ordering::Relaxed),
            deallocations: DEALLOCATIONS.load(Ordering::Relaxed),
            reallocations: REALLOCATIONS.load(Ordering::Relaxed),
            allocated_bytes: ALLOCATED_BYTES.load(Ordering::Relaxed),
            deallocated_bytes: DEALLOCATED_BYTES.load(Ordering::Relaxed),
            live_bytes: LIVE_BYTES.load(Ordering::Relaxed),
            peak_live_bytes: PEAK_LIVE_BYTES.load(Ordering::Relaxed),
            tracking_overflows: TRACKING_OVERFLOWS.load(Ordering::Relaxed),
        }
    }

    fn emit(self, scenario: &str, operations: usize) {
        eprintln!(
            "allocation-profile {scenario}: operations={operations}, allocations={allocations}, zeroed_allocations={zeroed_allocations}, reallocations={reallocations}, deallocations={deallocations}, allocated_bytes={allocated_bytes}, deallocated_bytes={deallocated_bytes}, live_bytes={live_bytes}, peak_live_bytes={peak_live_bytes}, tracking_overflows={tracking_overflows}",
            allocations = self.allocations,
            zeroed_allocations = self.zeroed_allocations,
            reallocations = self.reallocations,
            deallocations = self.deallocations,
            allocated_bytes = self.allocated_bytes,
            deallocated_bytes = self.deallocated_bytes,
            live_bytes = self.live_bytes,
            peak_live_bytes = self.peak_live_bytes,
            tracking_overflows = self.tracking_overflows,
        );
    }
}

struct CountingScope {
    epoch: u64,
    active: bool,
}

impl CountingScope {
    fn finish(mut self) -> AllocationSnapshot {
        self.stop();
        AllocationSnapshot::current()
    }

    fn stop(&mut self) {
        if !self.active {
            return;
        }
        let _ = ACTIVE_EPOCH.compare_exchange(self.epoch, 0, Ordering::AcqRel, Ordering::Acquire);
        while COUNTING_IN_FLIGHT.load(Ordering::Acquire) != 0 {
            spin_loop();
        }
        self.active = false;
    }
}

impl Drop for CountingScope {
    fn drop(&mut self) {
        self.stop();
    }
}

fn start_counting() -> CountingScope {
    assert_eq!(
        ACTIVE_EPOCH.load(Ordering::Acquire),
        0,
        "allocation profiles must be serialized"
    );
    ALLOCATIONS.store(0, Ordering::Relaxed);
    ZEROED_ALLOCATIONS.store(0, Ordering::Relaxed);
    DEALLOCATIONS.store(0, Ordering::Relaxed);
    REALLOCATIONS.store(0, Ordering::Relaxed);
    ALLOCATED_BYTES.store(0, Ordering::Relaxed);
    DEALLOCATED_BYTES.store(0, Ordering::Relaxed);
    LIVE_BYTES.store(0, Ordering::Relaxed);
    PEAK_LIVE_BYTES.store(0, Ordering::Relaxed);
    TRACKING_OVERFLOWS.store(0, Ordering::Relaxed);
    let mut epoch = NEXT_EPOCH.fetch_add(1, Ordering::Relaxed);
    if epoch == 0 {
        epoch = NEXT_EPOCH.fetch_add(1, Ordering::Relaxed);
    }
    ACTIVE_EPOCH.store(epoch, Ordering::Release);
    CountingScope {
        epoch,
        active: true,
    }
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

#[tokio::test(flavor = "current_thread")]
async fn allocation_scope_detects_a_retained_vector_and_peak() {
    let _profile_guard = PROFILE_LOCK.lock().await;
    let scope = start_counting();
    let retained = vec![7_u8; 8_192];
    std::hint::black_box(&retained);

    let snapshot = scope.finish();

    assert!(snapshot.live_bytes >= retained.capacity());
    assert!(snapshot.peak_live_bytes >= snapshot.live_bytes);
    assert_eq!(snapshot.tracking_overflows, 0);
    drop(retained);
}

#[test]
fn geometric_slope_verdict_distinguishes_growth_from_zero_or_bounded_plateau() {
    let growing = [
        (1, 32),
        (10, 320),
        (100, 3_200),
        (1_000, 32_000),
        (10_000, 320_000),
    ];
    let reclaimed = GEOMETRIC_CARDINALITIES.map(|cardinality| (cardinality, 0));
    let bounded = [(1, 1), (10, 4), (100, 4), (1_000, 4), (10_000, 4)];

    assert!(has_positive_retained_slope(&growing));
    assert!(!has_positive_retained_slope(&reclaimed));
    assert!(!has_positive_retained_slope(&bounded));
}

#[tokio::test(flavor = "current_thread")]
async fn allocation_scope_reports_released_vector_as_not_live() {
    let _profile_guard = PROFILE_LOCK.lock().await;
    let scope = start_counting();
    let released = vec![9_u8; 8_192];
    std::hint::black_box(&released);
    drop(released);

    let snapshot = scope.finish();

    assert_eq!(snapshot.live_bytes, 0);
    assert!(snapshot.peak_live_bytes >= 8_192);
    assert_eq!(snapshot.tracking_overflows, 0);
}

#[tokio::test(flavor = "current_thread")]
async fn prior_epoch_deallocation_is_not_charged_to_next_scope() {
    let _profile_guard = PROFILE_LOCK.lock().await;
    let first_scope = start_counting();
    let retained = vec![3_u8; 8_192];
    let first = first_scope.finish();
    assert!(first.live_bytes >= retained.capacity());

    let second_scope = start_counting();
    drop(retained);
    let second = second_scope.finish();

    assert_eq!(second.live_bytes, 0);
    assert_eq!(second.peak_live_bytes, 0);
    assert_eq!(second.tracking_overflows, 0);
}

#[tokio::test(flavor = "current_thread")]
async fn allocation_scope_drop_disables_counting_during_unwind() {
    let _profile_guard = PROFILE_LOCK.lock().await;
    let result = std::panic::catch_unwind(|| {
        let _scope = start_counting();
        let retained = vec![1_u8; 1_024];
        std::hint::black_box(retained);
        panic!("allocation scope unwind canary");
    });

    assert!(result.is_err());
    assert_eq!(ACTIVE_EPOCH.load(Ordering::Acquire), 0);
    assert_eq!(COUNTING_IN_FLIGHT.load(Ordering::Acquire), 0);
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
    let scope = start_counting();
    for _ in 0..operations {
        let cached: Option<AllocationValue> = cache.get("allocation:hot").await.unwrap();
        assert_eq!(cached, Some(AllocationValue::new(1)));
    }
    scope.finish().emit("hot-get-hits", operations);
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
    let scope = start_counting();
    for _ in 0..operations {
        assert!(cache.contains_key("allocation:contains").await);
    }
    scope.finish().emit("contains-key-hits", operations);
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
    let scope = start_counting();
    for _ in 0..operations {
        let cached: Option<AllocationValue> = no_subscriber
            .get("allocation:event:no-subscriber")
            .await
            .unwrap();
        assert_eq!(cached, Some(AllocationValue::new(10)));
    }
    scope
        .finish()
        .emit("event-preflight-no-subscriber", operations);

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
    let scope = start_counting();
    for _ in 0..operations {
        let cached: Option<AllocationValue> = mutation_subscriber
            .get("allocation:event:mutation-subscriber")
            .await
            .unwrap();
        assert_eq!(cached, Some(AllocationValue::new(11)));
    }
    scope
        .finish()
        .emit("event-preflight-mutation-subscriber", operations);

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
    let scope = start_counting();
    for _ in 0..operations {
        let cached: Option<AllocationValue> = access_subscriber
            .get("allocation:event:access-subscriber")
            .await
            .unwrap();
        assert_eq!(cached, Some(AllocationValue::new(12)));
    }
    scope
        .finish()
        .emit("event-preflight-access-subscriber", operations);
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
    let scope = start_counting();
    for _ in 0..operations {
        let cached = typed.get("typed-hot").await.unwrap();
        assert_eq!(cached, Some(AllocationValue::new(3)));
    }
    scope.finish().emit("typed-hot-get-hits", operations);
}

#[tokio::test]
#[ignore = "manual allocation profile; run with --ignored --nocapture"]
async fn profile_bulk_tag_invalidation() {
    let _profile_guard = PROFILE_LOCK.lock().await;
    let cache = HydraCache::local().max_capacity(1_000_000).build();
    let entries = 256;

    let scope = start_counting();
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
    let snapshot = scope.finish();

    assert_eq!(removed, entries as u64);
    snapshot.emit("bulk-tag-invalidation", entries);
}
