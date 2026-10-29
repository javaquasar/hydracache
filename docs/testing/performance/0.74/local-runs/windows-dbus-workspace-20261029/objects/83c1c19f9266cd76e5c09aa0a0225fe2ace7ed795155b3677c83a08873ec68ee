//! Separate allocator image: never links this allocator into the supervisor.
#[path = "../src/diagnostic_boot_document.rs"]
mod document;

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Counts {
    allocations: usize,
    requested: usize,
    deallocations: usize,
}
thread_local! {
    static ENABLED: Cell<bool> = const { Cell::new(false) };
    static COUNTS: Cell<Counts> = const { Cell::new(Counts {
        allocations: 0, requested: 0, deallocations: 0,
    }) };
}
struct Counter;
fn account(layout: Layout, allocation: bool) {
    // TLS may already be gone during thread teardown. The measured closure
    // enters both cells before enabling; accounting never allocates or panics.
    let _ = ENABLED.try_with(|enabled| {
        if enabled.get() {
            let _ = COUNTS.try_with(|cell| {
                let mut value = cell.get();
                if allocation {
                    value.allocations += 1;
                    value.requested += layout.size();
                } else {
                    value.deallocations += 1;
                }
                cell.set(value);
            });
        }
    });
}
unsafe impl GlobalAlloc for Counter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: forward the caller's unchanged allocator contract to System.
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            account(layout, true);
        }
        ptr
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: forward the caller's unchanged allocator contract to System.
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() {
            account(layout, true);
        }
        ptr
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        account(layout, false);
        // SAFETY: caller supplies the original pointer and matching layout.
        unsafe { System.dealloc(ptr, layout) };
    }
}
#[global_allocator]
static ALLOCATOR: Counter = Counter;
fn capture(f: impl FnOnce()) -> Counts {
    assert!(!ENABLED.get(), "nested allocator capture is forbidden");
    COUNTS.set(Counts::default());
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            ENABLED.set(false);
        }
    }
    ENABLED.set(true);
    let guard = Guard;
    f();
    drop(guard);
    COUNTS.get()
}
const BOOT: &[u8; 37] = b"e81671d6-bf23-4ebf-b8fe-5dad1f45866f\n";
fn fill(bytes: &mut [u8]) -> std::io::Result<usize> {
    std::hint::black_box(&mut *bytes)[..BOOT.len()].copy_from_slice(BOOT);
    Ok(BOOT.len())
}
fn old_read() {
    // Exact temporary algorithm from bounded at the preregistered baseline;
    // source contracts bind this code to that implementation separately.
    let mut bytes = vec![0; 65_537];
    let count = fill(&mut bytes).unwrap();
    assert!(count > 0 && count <= 65_536);
    bytes.truncate(count);
    assert_eq!(&bytes[..], BOOT);
    std::hint::black_box(bytes);
}
fn new_read() {
    let (bytes, count) = document::read_once(fill).unwrap();
    assert_eq!(&bytes[..count], BOOT);
    std::hint::black_box(bytes);
}
#[test]
fn fixed_boot_reader_uses_one_small_read() {
    let (_, count) = document::read_once(|bytes| {
        assert_eq!(bytes.len(), 38);
        fill(bytes)
    })
    .unwrap();
    assert_eq!(count, 37);
}
#[test]
fn fixed_boot_reader_allocation_pairs() {
    for scope in 0..5 {
        let (baseline, candidate) = if scope % 2 == 0 {
            (
                capture(|| (0..32).for_each(|_| old_read())),
                capture(|| (0..32).for_each(|_| new_read())),
            )
        } else {
            let candidate = capture(|| (0..32).for_each(|_| new_read()));
            (capture(|| (0..32).for_each(|_| old_read())), candidate)
        };
        println!("HC074-BOOT-ALLOCATION scope={scope} reads=32 baseline_allocations={} baseline_requested={} baseline_deallocations={} candidate_allocations={} candidate_requested={} candidate_deallocations={} whole_worker=false", baseline.allocations, baseline.requested, baseline.deallocations, candidate.allocations, candidate.requested, candidate.deallocations);
        assert_eq!(
            baseline,
            Counts {
                allocations: 32,
                requested: 32 * 65_537,
                deallocations: 32
            }
        );
        assert_eq!(candidate, Counts::default());
    }
}
#[test]
fn allocator_capture_restores_on_panic_and_refuses_nesting() {
    assert!(std::panic::catch_unwind(|| capture(|| {
        capture(|| ());
    }))
    .is_err());
    assert!(!ENABLED.get());
    assert_eq!(capture(|| ()), Counts::default());
    assert!(std::panic::catch_unwind(|| capture(|| panic!("injected"))).is_err());
    assert!(!ENABLED.get());
}
#[test]
fn allocator_capture_is_thread_local() {
    let worker = std::thread::spawn(|| capture(old_read));
    assert_eq!(capture(|| ()), Counts::default());
    assert_eq!(worker.join().unwrap().allocations, 1);
}
