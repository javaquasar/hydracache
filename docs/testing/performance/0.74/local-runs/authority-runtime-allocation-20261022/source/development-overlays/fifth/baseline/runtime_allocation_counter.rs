//! Successful System layout requests only, including old-layout release on realloc.
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(crate) struct Counts {
    pub allocations: u64,
    pub zeroed: u64,
    pub reallocations: u64,
    pub requested_bytes: u64,
    pub deallocations: u64,
    pub released_bytes: u64,
    pub peak_delta: i64,
}
impl Counts {
    pub fn live(self) -> i64 {
        self.requested_bytes as i64 - self.released_bytes as i64
    }
}
#[derive(Clone, Copy, Default)]
struct State {
    active: bool,
    counts: Counts,
}
thread_local! { static STATE: Cell<State> = const { Cell::new(State {
    active:false, counts:Counts { allocations:0,zeroed:0,reallocations:0,
    requested_bytes:0,deallocations:0,released_bytes:0,peak_delta:0 }
}) }; }
fn record(size: usize, old: usize, kind: u8) {
    let _ = STATE.try_with(|cell| {
        let mut state = cell.get();
        if !state.active {
            return;
        }
        let count = &mut state.counts;
        if kind == 3 {
            count.deallocations = count.deallocations.wrapping_add(1);
            count.released_bytes = count.released_bytes.wrapping_add(size as u64);
        } else {
            count.requested_bytes = count.requested_bytes.wrapping_add(size as u64);
            if kind == 2 {
                count.reallocations = count.reallocations.wrapping_add(1);
                count.released_bytes = count.released_bytes.wrapping_add(old as u64);
            } else {
                count.allocations = count.allocations.wrapping_add(1);
                count.zeroed = count.zeroed.wrapping_add(u64::from(kind == 1));
            }
        }
        count.peak_delta = count.peak_delta.max(count.live());
        cell.set(state);
    });
}
struct Counter;
// SAFETY: forwarding preserves System's exact allocation contracts. Accounting
// uses only constant-initialized thread-local Cells: no allocation, lock or panic.
unsafe impl GlobalAlloc for Counter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            record(layout.size(), 0, 0);
        }
        pointer
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            record(layout.size(), 0, 1);
        }
        pointer
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let pointer = unsafe { System.realloc(pointer, layout, size) };
        if !pointer.is_null() {
            record(size, layout.size(), 2);
        }
        pointer
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        record(layout.size(), 0, 3);
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
pub(crate) fn capture<T>(run: impl FnOnce() -> T) -> (T, Counts) {
    let previous = STATE.with(|state| {
        assert!(
            !state.get().active,
            "nested capture is not a lifetime cohort"
        );
        state.replace(State {
            active: true,
            counts: Counts::default(),
        })
    });
    let _restore = Restore(previous);
    let value = run();
    (value, STATE.with(|state| state.get().counts))
}

#[test]
fn counter_realloc_releases_old_layout_and_reconciles_drop() {
    let (_, count) = capture(|| unsafe {
        let first = Layout::from_size_align(16, 8).unwrap();
        let pointer = ALLOCATOR.alloc(first);
        assert!(!pointer.is_null());
        let pointer = ALLOCATOR.realloc(pointer, first, 32);
        assert!(!pointer.is_null());
        ALLOCATOR.dealloc(pointer, Layout::from_size_align(32, 8).unwrap());
    });
    assert_eq!(count.allocations, 1);
    assert_eq!(count.reallocations, 1);
    assert_eq!(count.requested_bytes, 48);
    assert_eq!(count.released_bytes, 48);
    assert_eq!(count.peak_delta, 32);
    assert_eq!(count.live(), 0);
}
#[test]
fn counter_cross_capture_lifetime_and_zeroed_forwarding() {
    let (bytes, allocated) = capture(|| std::hint::black_box(vec![0_u8; 65]));
    assert_eq!(allocated.allocations, 1);
    assert_eq!(allocated.zeroed, 1);
    assert_eq!(allocated.live(), 65);
    let (_, released) = capture(|| drop(bytes));
    assert_eq!(released.allocations, 0);
    assert_eq!(released.live(), -65);
    assert_eq!(allocated.live() + released.live(), 0);
}
#[test]
fn counter_unwind_restores_inactive_state_and_does_not_count_other_threads() {
    assert!(std::panic::catch_unwind(|| capture(|| panic!("owned counter control"))).is_err());
    assert!(!STATE.with(|state| state.get().active));
    let worker = std::thread::spawn(|| capture(|| std::hint::black_box(vec![1_u8; 64])).1);
    assert_eq!(capture(|| ()).1, Counts::default());
    assert_eq!(worker.join().unwrap().live(), 64);
}
