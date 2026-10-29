//! Tool-only System allocator. It is never linked into product binaries.
//! Live bytes are outstanding requested layouts, not allocator active/resident
//! bytes. Realloc records the post-success requested size, not System's hidden
//! old/new overlap. This profiling lane must not certify CPU/goodput/p99.
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

pub struct Allocator;
static LIVE: AtomicU64 = AtomicU64::new(0);
static PEAK: AtomicU64 = AtomicU64::new(0);
static GROSS: AtomicU64 = AtomicU64::new(0);
static CALLS: AtomicU64 = AtomicU64::new(0);
static ACTIVE: AtomicBool = AtomicBool::new(false);

fn allocated(size: usize) {
    let live = LIVE.fetch_add(size as u64, Ordering::SeqCst) + size as u64;
    if ACTIVE.load(Ordering::SeqCst) {
        GROSS.fetch_add(size as u64, Ordering::SeqCst);
        CALLS.fetch_add(1, Ordering::SeqCst);
        PEAK.fetch_max(live, Ordering::SeqCst);
    }
}

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: unchanged layout delegated to the same System allocator.
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            allocated(layout.size());
        }
        pointer
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: unchanged layout delegated to System.
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            allocated(layout.size());
        }
        pointer
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: original pointer/layout delegated to their backing allocator.
        unsafe { System.dealloc(pointer, layout) };
        LIVE.fetch_sub(layout.size() as u64, Ordering::SeqCst);
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        // SAFETY: System owns the original allocation; arguments pass unchanged.
        let new_pointer = unsafe { System.realloc(pointer, layout, size) };
        if !new_pointer.is_null() {
            LIVE.fetch_sub(layout.size() as u64, Ordering::SeqCst);
            allocated(size);
        }
        new_pointer
    }
}

pub fn live() -> u64 {
    LIVE.load(Ordering::SeqCst)
}

pub struct Scope {
    pub before: u64,
}

#[derive(Debug, serde::Serialize)]
pub struct Measurement {
    pub gross_allocated_bytes: u64,
    pub successful_allocation_calls: u64,
    pub live_before_bytes: u64,
    pub live_after_bytes: u64,
    pub peak_live_requested_bytes: u64,
    pub peak_live_above_start_bytes: u64,
}

impl Scope {
    // Dedicated current-thread runtime; no other tasks/measurements are active.
    pub fn start() -> Self {
        assert!(!ACTIVE.load(Ordering::SeqCst));
        let before = live();
        GROSS.store(0, Ordering::SeqCst);
        CALLS.store(0, Ordering::SeqCst);
        PEAK.store(before, Ordering::SeqCst);
        ACTIVE.store(true, Ordering::SeqCst);
        Self { before }
    }
    pub fn finish(self) -> Measurement {
        ACTIVE.store(false, Ordering::SeqCst);
        Measurement {
            gross_allocated_bytes: GROSS.load(Ordering::SeqCst),
            successful_allocation_calls: CALLS.load(Ordering::SeqCst),
            live_before_bytes: self.before,
            live_after_bytes: live(),
            peak_live_requested_bytes: PEAK.load(Ordering::SeqCst),
            peak_live_above_start_bytes: PEAK.load(Ordering::SeqCst).saturating_sub(self.before),
        }
    }
}
impl Drop for Scope {
    fn drop(&mut self) {
        ACTIVE.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn epoch_counts_gross_calls_peak_and_owner_release_without_oracle_allocation() {
        let scope = Scope::start();
        let before = scope.before;
        let first_layout = Layout::from_size_align(4096, 8).unwrap();
        let second_layout = Layout::from_size_align(8192, 8).unwrap();
        // SAFETY: successful allocations are released with their own layouts.
        unsafe {
            let first = Allocator.alloc(first_layout);
            let second = Allocator.alloc_zeroed(second_layout);
            assert!(!first.is_null() && !second.is_null());
            Allocator.dealloc(first, first_layout);
            Allocator.dealloc(second, second_layout);
        }
        let measured = scope.finish();
        assert_eq!(measured.gross_allocated_bytes, 12288);
        assert_eq!(measured.successful_allocation_calls, 2);
        assert_eq!(measured.peak_live_above_start_bytes, 12288);
        assert_eq!(measured.live_after_bytes, before);
    }
    #[test]
    fn allocation_zeroed_reallocation_and_drop_preserve_live_accounting() {
        // No measurement epoch in parallel tests: test only this layout's delta.
        let layout = Layout::from_size_align(4096, 8).unwrap();
        // SAFETY: every pointer is paired with its successful allocation layout.
        unsafe {
            let pointer = Allocator.alloc_zeroed(layout);
            assert!(!pointer.is_null());
            assert_eq!(*pointer, 0);
            let before = live();
            let pointer = Allocator.realloc(pointer, layout, 8192);
            assert!(!pointer.is_null());
            assert_eq!(live(), before + 4096);
            Allocator.dealloc(pointer, Layout::from_size_align(8192, 8).unwrap());
            assert_eq!(live(), before - 4096);
        }
    }
}
