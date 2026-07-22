//! Process-wide gross allocation measurement for dedicated performance lanes.
//!
//! The allocator counts bytes requested by successful allocations. A successful
//! reallocation contributes its complete new size, rather than only the delta,
//! so the result is deliberately a gross-allocation metric. Because a Rust
//! process has only one global allocator, callers must run these measurements
//! on an otherwise quiescent process if they need workload-only evidence.

use std::alloc::{GlobalAlloc, Layout, System};
use std::future::Future;
use std::hint::spin_loop;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use tokio::sync::Mutex;

struct CountingAllocator;

static ACTIVE_EPOCH: AtomicU64 = AtomicU64::new(0);
static NEXT_EPOCH: AtomicU64 = AtomicU64::new(1);
static COUNTING_IN_FLIGHT: AtomicUsize = AtomicUsize::new(0);
static GROSS_ALLOCATED_BYTES: AtomicUsize = AtomicUsize::new(0);
static MEASUREMENT_LOCK: Mutex<()> = Mutex::const_new(());

#[global_allocator]
static GLOBAL_ALLOCATOR: CountingAllocator = CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: The request is delegated to the platform allocator without
        // changing the layout or otherwise manufacturing allocation metadata.
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            record_successful_allocation(layout.size());
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: The request is delegated to the platform allocator without
        // changing the layout or otherwise manufacturing allocation metadata.
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            record_successful_allocation(layout.size());
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: `pointer` and `layout` are passed through exactly as supplied
        // by the caller of `GlobalAlloc::dealloc` to the same backing allocator.
        unsafe { System.dealloc(pointer, layout) };
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: The original pointer, layout, and requested new size are
        // delegated unchanged to the same platform allocator.
        let new_pointer = unsafe { System.realloc(pointer, layout, new_size) };
        if !new_pointer.is_null() {
            record_successful_allocation(new_size);
        }
        new_pointer
    }
}

fn record_successful_allocation(bytes: usize) {
    let epoch = ACTIVE_EPOCH.load(Ordering::Acquire);
    if epoch == 0 {
        return;
    }

    COUNTING_IN_FLIGHT.fetch_add(1, Ordering::AcqRel);
    // The epoch recheck closes the gap between observing an active scope and
    // publishing this allocator callback as in-flight. A callback delayed
    // across scope boundaries must never charge the next measurement.
    if ACTIVE_EPOCH.load(Ordering::Acquire) == epoch {
        let _ =
            GROSS_ALLOCATED_BYTES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                Some(current.saturating_add(bytes))
            });
    }
    COUNTING_IN_FLIGHT.fetch_sub(1, Ordering::Release);
}

/// Gross bytes attributed to one serialized workload measurement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AllocationMeasurement {
    /// Logical operations completed by the measured workload.
    pub operations: u64,
    /// Sum of successful allocation sizes, including full reallocation sizes.
    pub gross_allocated_bytes: u64,
    /// Gross allocated bytes divided by logical operations.
    pub gross_allocated_bytes_per_operation: f64,
}

struct CountingScope {
    active: bool,
    epoch: u64,
}

impl CountingScope {
    fn start() -> Self {
        debug_assert_eq!(ACTIVE_EPOCH.load(Ordering::Acquire), 0);
        GROSS_ALLOCATED_BYTES.store(0, Ordering::Relaxed);
        let mut epoch = NEXT_EPOCH.fetch_add(1, Ordering::Relaxed);
        if epoch == 0 {
            epoch = NEXT_EPOCH.fetch_add(1, Ordering::Relaxed);
        }
        ACTIVE_EPOCH.store(epoch, Ordering::Release);
        Self {
            active: true,
            epoch,
        }
    }

    fn finish(mut self, operations: u64) -> AllocationMeasurement {
        let gross_allocated_bytes = self.stop();
        AllocationMeasurement {
            operations,
            gross_allocated_bytes,
            gross_allocated_bytes_per_operation: gross_allocated_bytes as f64 / operations as f64,
        }
    }

    fn stop(&mut self) -> u64 {
        if !self.active {
            return 0;
        }

        let _ = ACTIVE_EPOCH.compare_exchange(self.epoch, 0, Ordering::AcqRel, Ordering::Acquire);
        while COUNTING_IN_FLIGHT.load(Ordering::Acquire) != 0 {
            spin_loop();
        }
        self.active = false;

        u64::try_from(GROSS_ALLOCATED_BYTES.load(Ordering::Relaxed))
            .expect("gross allocation byte count must fit into u64")
    }
}

impl Drop for CountingScope {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

/// Measures gross allocation bytes while polling `workload` to completion.
///
/// Measurements are serialized process-wide. Work performed by unrelated tasks
/// while `workload` is running is also visible to the global allocator, so the
/// caller is responsible for providing a quiescent dedicated runner. The output
/// of `workload` is returned unchanged so callers can validate operation results.
/// If this future is cancelled or unwinds, counting is disabled by an RAII guard.
///
/// # Panics
///
/// Panics when `operations` is zero because bytes-per-operation would be
/// undefined.
pub async fn measure_allocations<F>(
    operations: u64,
    workload: F,
) -> (F::Output, AllocationMeasurement)
where
    F: Future,
{
    assert!(
        operations > 0,
        "allocation measurement requires operations > 0"
    );

    let _measurement_guard = MEASUREMENT_LOCK.lock().await;
    let scope = CountingScope::start();
    let output = workload.await;
    let measurement = scope.finish(operations);

    (output, measurement)
}
