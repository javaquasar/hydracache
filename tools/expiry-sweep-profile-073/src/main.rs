use std::alloc::{GlobalAlloc, Layout, System};
use std::error::Error;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use hydracache_client_transport_axum::performance_profile::{
    ExpirySweepProfileFixture, ExpirySweepProfileObservation, ExpirySweepProfileScenario,
};
use serde::Serialize;

struct CountingAllocator;

static COUNTING: AtomicBool = AtomicBool::new(false);
static ALLOCATION_COUNT: AtomicU64 = AtomicU64::new(0);
static GROSS_ALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);

#[global_allocator]
static GLOBAL_ALLOCATOR: CountingAllocator = CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: allocation is delegated unchanged to the system allocator.
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            record_allocation(layout.size());
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: allocation is delegated unchanged to the system allocator.
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            record_allocation(layout.size());
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: pointer and layout are delegated to the allocator that created them.
        unsafe { System.dealloc(pointer, layout) };
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: reallocation is delegated unchanged to the system allocator.
        let new_pointer = unsafe { System.realloc(pointer, layout, new_size) };
        if !new_pointer.is_null() {
            record_allocation(new_size);
        }
        new_pointer
    }
}

fn record_allocation(bytes: usize) {
    if COUNTING.load(Ordering::Relaxed) {
        ALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);
        GROSS_ALLOCATED_BYTES.fetch_add(bytes as u64, Ordering::Relaxed);
    }
}

#[derive(Serialize)]
struct ProfileResult {
    schema_version: u32,
    profile_id: &'static str,
    scenario: &'static str,
    fixture_entries: usize,
    scan_limit: usize,
    identity_bytes_per_key: usize,
    examined_keys: usize,
    expired_keys: usize,
    cloned_keys: usize,
    cloned_identity_bytes: usize,
    next_cursor_present: bool,
    allocation_count: u64,
    gross_allocated_bytes: u64,
}

fn main() -> Result<(), Box<dyn Error>> {
    let scenario_name = std::env::args()
        .nth(1)
        .ok_or("usage: expiry-sweep-profile-073 <scenario>")?;
    let scenario = ExpirySweepProfileScenario::parse(&scenario_name)
        .ok_or_else(|| format!("unsupported scenario: {scenario_name}"))?;
    let fixture = ExpirySweepProfileFixture::new(scenario);
    let (observation, allocation_count, gross_allocated_bytes) = measure(|| fixture.run());
    validate_observation(scenario, observation)?;
    let result = ProfileResult {
        schema_version: 1,
        profile_id: "w2-expiry-sweep-073-v1",
        scenario: scenario.as_str(),
        fixture_entries: 512,
        scan_limit: 256,
        identity_bytes_per_key: 96,
        examined_keys: observation.examined_keys,
        expired_keys: observation.expired_keys,
        cloned_keys: observation.cloned_keys,
        cloned_identity_bytes: observation.cloned_identity_bytes,
        next_cursor_present: observation.next_cursor_present,
        allocation_count,
        gross_allocated_bytes,
    };
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}

fn measure<T>(work: impl FnOnce() -> T) -> (T, u64, u64) {
    ALLOCATION_COUNT.store(0, Ordering::Relaxed);
    GROSS_ALLOCATED_BYTES.store(0, Ordering::Relaxed);
    assert!(!COUNTING.swap(true, Ordering::SeqCst));
    let result = work();
    COUNTING.store(false, Ordering::SeqCst);
    (
        result,
        ALLOCATION_COUNT.load(Ordering::Relaxed),
        GROSS_ALLOCATED_BYTES.load(Ordering::Relaxed),
    )
}

fn validate_observation(
    scenario: ExpirySweepProfileScenario,
    observation: ExpirySweepProfileObservation,
) -> Result<(), Box<dyn Error>> {
    let expected_expired = match scenario {
        ExpirySweepProfileScenario::NoneExpired
        | ExpirySweepProfileScenario::CursorWrapNoneExpired => 0,
        ExpirySweepProfileScenario::HalfExpired => 128,
        ExpirySweepProfileScenario::AllExpired => 256,
    };
    let expected_clones = expected_expired + 1;
    if observation.examined_keys != 256
        || observation.expired_keys != expected_expired
        || observation.cloned_keys != expected_clones
        || observation.cloned_identity_bytes != expected_clones * 96
        || !observation.next_cursor_present
    {
        return Err(format!("unexpected logical observation: {observation:?}").into());
    }
    Ok(())
}
