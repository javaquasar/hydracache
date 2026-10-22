//! Isolated test-image allocator. Counts requests, never elapsed time or RSS.
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Counts {
    pub allocations: usize,
    pub zeroed: usize,
    pub reallocations: usize,
    pub requested_bytes: usize,
    pub deallocations: usize,
    pub deallocated_bytes: usize,
}
impl Counts {
    pub fn json(self) -> String {
        format!("{{\"allocations\":{},\"zeroed\":{},\"reallocations\":{},\"requested_bytes\":{},\"deallocations\":{},\"deallocated_bytes\":{}}}",self.allocations,self.zeroed,self.reallocations,self.requested_bytes,self.deallocations,self.deallocated_bytes)
    }
}
#[derive(Clone, Copy, Default)]
struct State {
    active: bool,
    counts: Counts,
}
thread_local! {
    static STATE: Cell<State> = const { Cell::new(State { active: false, counts: Counts {
        allocations: 0, zeroed: 0, reallocations: 0, requested_bytes: 0,
        deallocations: 0, deallocated_bytes: 0,
    } }) };
}
fn record(bytes: usize, kind: u8) {
    let _ = STATE.try_with(|cell| {
        let mut state = cell.get();
        if !state.active {
            return;
        }
        match kind {
            0 | 1 => {
                state.counts.allocations = state.counts.allocations.wrapping_add(1);
                state.counts.zeroed = state.counts.zeroed.wrapping_add(usize::from(kind == 1));
                state.counts.requested_bytes = state.counts.requested_bytes.wrapping_add(bytes);
            }
            2 => {
                state.counts.reallocations = state.counts.reallocations.wrapping_add(1);
                state.counts.requested_bytes = state.counts.requested_bytes.wrapping_add(bytes);
            }
            _ => {
                state.counts.deallocations = state.counts.deallocations.wrapping_add(1);
                state.counts.deallocated_bytes = state.counts.deallocated_bytes.wrapping_add(bytes);
            }
        }
        cell.set(state);
    });
}
struct Counter;
// SAFETY: unchanged System allocator contracts; accounting uses only initialized
// thread-local Cells and cannot allocate, lock or invoke user callbacks.
unsafe impl GlobalAlloc for Counter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            record(layout.size(), 0);
        }
        pointer
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            record(layout.size(), 1);
        }
        pointer
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, bytes: usize) -> *mut u8 {
        let pointer = unsafe { System.realloc(pointer, layout, bytes) };
        if !pointer.is_null() {
            record(bytes, 2);
        }
        pointer
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        record(layout.size(), 3);
        unsafe { System.dealloc(pointer, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: Counter = Counter;
struct Restore(State);
impl Drop for Restore {
    fn drop(&mut self) {
        let _ = STATE.try_with(|state| state.set(self.0));
    }
}
pub(super) fn capture<T>(run: impl FnOnce() -> T) -> (T, Counts) {
    let previous = STATE.with(|state| {
        state.replace(State {
            active: true,
            counts: Counts::default(),
        })
    });
    let _restore = Restore(previous);
    let value = run();
    (value, STATE.get().counts)
}

#[test]
fn counter_nested_unwind_and_thread_isolation() {
    let (_, outer) = capture(|| {
        let (bytes, inner) = capture(|| std::hint::black_box(vec![7_u8; 128]));
        assert_eq!(inner.allocations, 1);
        assert_eq!(inner.requested_bytes, 128);
        // Drop belongs to the outer capture, not the inner allocation cohort.
        drop(bytes);
    });
    assert_eq!(outer.allocations, 0);
    assert_eq!(outer.deallocated_bytes, 128);
    assert!(std::panic::catch_unwind(|| capture(|| panic!("counter unwind control"))).is_err());
    assert!(!STATE.get().active);
    let worker = std::thread::spawn(|| capture(|| std::hint::black_box(vec![0_u8; 64])).1);
    assert_eq!(capture(|| ()).1, Counts::default());
    let other = worker.join().unwrap();
    assert_eq!(other.allocations, 1);
    assert_eq!(other.zeroed, 1);
    assert_eq!(other.requested_bytes, 64);
}

#[test]
fn counter_zeroed_reallocation_and_drop_forwarding() {
    let (_, counts) = capture(|| {
        let mut bytes = std::hint::black_box(vec![0_u8; 4]);
        bytes.push(1);
        assert_eq!(std::hint::black_box(&bytes)[4], 1);
    });
    assert_eq!(counts.allocations, 1);
    assert_eq!(counts.zeroed, 1);
    assert_eq!(counts.reallocations, 1);
    assert_eq!(counts.requested_bytes, 12);
    assert_eq!(counts.deallocations, 1);
    assert_eq!(counts.deallocated_bytes, 8);
    assert_eq!(capture(|| ()).1, Counts::default());
}
