use std::alloc::{GlobalAlloc, Layout, System};
use std::error::Error;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use bytes::Bytes;
use hydracache_client_hc2::wire::CacheEvent;
use serde::Serialize;

const KEY_BYTES: usize = 64;

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
    scenario: String,
    fanout: usize,
    key_bytes: usize,
    value_bytes: usize,
    frames_retained: usize,
    logical_copied_bytes: usize,
    allocation_count: u64,
    gross_allocated_bytes: u64,
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let fanout = args
        .next()
        .ok_or("usage: hc2-event-copy-profile-073 <fanout> <value-bytes>")?
        .parse::<usize>()?;
    let value_bytes = args
        .next()
        .ok_or("usage: hc2-event-copy-profile-073 <fanout> <value-bytes>")?
        .parse::<usize>()?;
    if args.next().is_some() || !matches!(fanout, 1 | 8 | 16) || !matches!(value_bytes, 128 | 4096)
    {
        return Err("unsupported frozen W5 profile shape".into());
    }

    let key = Bytes::from(vec![0x6b; KEY_BYTES]);
    let value = Bytes::from(vec![0x76; value_bytes]);
    let mut frames = Vec::with_capacity(fanout);
    let ((), allocation_count, gross_allocated_bytes) = measure(|| {
        for subscription_id in 0..fanout {
            frames.push(CacheEvent {
                subscription_id: subscription_id as u64,
                watermark: 1,
                key: Bytes::copy_from_slice(&key),
                value: Bytes::copy_from_slice(&value),
                removed: false,
            });
        }
    });
    std::hint::black_box(&frames);

    let result = ProfileResult {
        schema_version: 1,
        profile_id: "w5-hc2-event-copy-073-v1",
        scenario: format!("fanout-{fanout}-value-{value_bytes}"),
        fanout,
        key_bytes: KEY_BYTES,
        value_bytes,
        frames_retained: frames.len(),
        logical_copied_bytes: fanout * (KEY_BYTES + value_bytes),
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
