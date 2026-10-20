//! Opt-in test binary only. Rust requests, not libc malloc, CPU, RSS or authority.
use super::*;

#[path = "../tests/support/authority_hex_alloc.rs"]
mod authority_hex_alloc;

#[path = "../tests/support/validation_leaf_requests.rs"]
mod validation_leaf_requests;

#[path = "../tests/support/projection_alloc.rs"]
mod projection_fields;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

#[path = "diagnostic_boot_comparison_alloc_tests.rs"]
mod boot_comparison;

#[path = "../tests/support/projection_prefix_alloc.rs"]
mod projection_prefix;

#[path = "../tests/support/fresh_status_alloc.rs"]
mod fresh_status;

#[path = "../tests/support/pid_path_alloc.rs"]
mod pid_path;

#[path = "../tests/support/document_scratch_alloc.rs"]
mod document_scratch;

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, serde::Serialize)]
struct Counts {
    allocations: u64,
    reallocations: u64,
    requested_bytes: u64,
}
#[derive(Clone, Copy, Default)]
struct State {
    active: bool,
    counts: Counts,
}
thread_local! { static COUNTS: Cell<State> = const { Cell::new(State { active:false, counts: Counts { allocations:0, reallocations:0, requested_bytes:0 } }) }; }
fn record(bytes: usize, reallocate: bool) {
    let _ = COUNTS.try_with(|counter| {
        let mut state = counter.get();
        if state.active {
            if reallocate {
                state.counts.reallocations = state.counts.reallocations.wrapping_add(1);
            } else {
                state.counts.allocations = state.counts.allocations.wrapping_add(1);
            }
            state.counts.requested_bytes = state.counts.requested_bytes.wrapping_add(bytes as u64);
            counter.set(state);
        }
    });
}
struct Counter;
// SAFETY: all allocation operations forward unchanged to System. Observations use
// only constant-initialized thread-local Cells, no heap, locks or unwinding.
unsafe impl GlobalAlloc for Counter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            record(layout.size(), false);
        }
        pointer
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            record(layout.size(), false);
        }
        pointer
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, bytes: usize) -> *mut u8 {
        let pointer = unsafe { System.realloc(pointer, layout, bytes) };
        if !pointer.is_null() {
            record(bytes, true);
        }
        pointer
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: Counter = Counter;
struct Restore(State);
impl Drop for Restore {
    fn drop(&mut self) {
        let _ = COUNTS.try_with(|state| state.set(self.0));
    }
}
fn capture<T>(run: impl FnOnce() -> T) -> (T, Counts) {
    let previous = COUNTS.with(|state| {
        state.replace(State {
            active: true,
            counts: Counts::default(),
        })
    });
    let _restore = Restore(previous);
    let value = run();
    (value, COUNTS.with(|state| state.get().counts))
}
#[test]
fn fixed_leaf_alloc_counter_restores_nested_unwind_and_thread_state() {
    let (_, counts) = capture(|| {
        let (value, inner) = capture(|| std::hint::black_box(vec![7_u8; 128]));
        assert_eq!(inner.allocations, 1);
        assert_eq!(inner.requested_bytes, 128);
        drop(value);
    });
    assert_eq!(counts, Counts::default());
    assert!(
        std::panic::catch_unwind(|| capture(|| panic!("allocation capture unwind canary")))
            .is_err()
    );
    assert!(!COUNTS.with(|s| s.get().active));
    let other = std::thread::spawn(|| {
        assert!(!COUNTS.with(|s| s.get().active));
        capture(|| std::hint::black_box(vec![3_u8; 64])).1
    })
    .join()
    .unwrap();
    assert_eq!(other.allocations, 1);
    assert_eq!(other.requested_bytes, 64);
    assert_eq!(capture(|| {}).1, Counts::default());
}
#[test]
fn fixed_leaf_alloc_counter_zeroed_reallocation_and_inactive_forwarding() {
    record(999, false);
    let (_, counts) = capture(|| {
        let layout = Layout::from_size_align(16, 8).unwrap();
        // SAFETY: exact live System allocation/layout, grown once and freed once.
        unsafe {
            let pointer = ALLOCATOR.alloc_zeroed(layout);
            assert!(!pointer.is_null());
            assert!(std::slice::from_raw_parts(pointer, 16)
                .iter()
                .all(|b| *b == 0));
            let pointer = ALLOCATOR.realloc(pointer, layout, 32);
            assert!(!pointer.is_null());
            assert!(std::slice::from_raw_parts(pointer, 16)
                .iter()
                .all(|b| *b == 0));
            ALLOCATOR.dealloc(pointer, Layout::from_size_align(32, 8).unwrap());
        }
    });
    assert_eq!(counts.allocations, 1);
    assert_eq!(counts.reallocations, 1);
    assert_eq!(counts.requested_bytes, 48);
    assert_eq!(capture(|| {}).1, Counts::default());
}
#[test]
#[ignore = "manual opt-in allocation requests through actual kernel leaf calls; not timing or admission"]
fn fixed_leaf_path_allocation_attribution() {
    assert_ne!(unsafe { libc::geteuid() }, 0);
    let origin = Origin::open().unwrap();
    for _ in 0..32 {
        boot(&origin.root).unwrap();
        reference_boot(&origin.root).unwrap();
        for name in NAMES {
            namespace(name).unwrap();
            reference_namespace(name).unwrap();
        }
    }
    for sample in 0..3 {
        for leaf in ["boot", "namespace"] {
            let (_, reference) = capture(|| {
                for i in 0..1024 {
                    if leaf == "boot" {
                        std::hint::black_box(reference_boot(&origin.root)).unwrap();
                    } else {
                        std::hint::black_box(reference_namespace(NAMES[i % 4])).unwrap();
                    }
                }
            });
            let (_, candidate) = capture(|| {
                for i in 0..1024 {
                    if leaf == "boot" {
                        std::hint::black_box(boot(&origin.root)).unwrap();
                    } else {
                        std::hint::black_box(namespace(NAMES[i % 4])).unwrap();
                    }
                }
            });
            println!(
                "HC074-LEAF-ALLOC {}",
                serde_json::json!({"sample":sample,"leaf":leaf,"iterations":1024,"reference":reference,"candidate":candidate,"rust_requests_only":true})
            );
        }
    }
    origin.revalidate().unwrap();
    println!("HC074-LEAF-ALLOC-CLOSED timer_changed=false server=false product=false");
}
