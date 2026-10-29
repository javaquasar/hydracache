//! Test-only, single-thread attribution of the exact existing proposal.
use super::*;
use serde_json::json;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::hint::black_box;
#[cfg(windows)]
use std::time::Instant;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
struct Counts {
    allocs: u64,
    reallocs: u64,
    requested: u64,
    live: i64,
    peak: i64,
}

thread_local! {
    static ACTIVE: Cell<bool> = const { Cell::new(false) };
    static COUNTS: Cell<Counts> = const { Cell::new(Counts {
        allocs: 0, reallocs: 0, requested: 0, live: 0, peak: 0
    }) };
}

fn update(add: usize, remove: usize, alloc: bool, realloc: bool) {
    let _ = ACTIVE.try_with(|active| {
        if active.get() {
            let _ = COUNTS.try_with(|cell| {
                let mut c = cell.get();
                c.allocs += u64::from(alloc);
                c.reallocs += u64::from(realloc);
                c.requested += add as u64;
                c.live += add as i64 - remove as i64;
                c.peak = c.peak.max(c.live);
                cell.set(c);
            });
        }
    });
}

struct Meter;
// SAFETY: every operation delegates unchanged pointer/layout/alignment to System.
// Only successful allocations change accounting. The TLS cells allocate nothing;
// try_with tolerates thread teardown and no pointer is retained by the observer.
unsafe impl GlobalAlloc for Meter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            update(layout.size(), 0, true, false);
        }
        ptr
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() {
            update(layout.size(), 0, true, false);
        }
        ptr
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        update(0, layout.size(), false, false);
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let new_ptr = unsafe { System.realloc(ptr, layout, size) };
        if !new_ptr.is_null() {
            update(size, layout.size(), false, true);
        }
        new_ptr
    }
}

#[global_allocator]
static ALLOCATOR: Meter = Meter;

fn measured<T>(f: impl FnOnce() -> T) -> (T, Counts) {
    ACTIVE.with(|active| assert!(!active.get()));
    COUNTS.with(|cell| cell.set(Counts::default()));
    ACTIVE.with(|active| active.set(true));
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            ACTIVE.with(|active| active.set(false));
        }
    }
    let reset = Reset;
    let result = f();
    drop(reset);
    let counts = COUNTS.with(Cell::get);
    (result, counts)
}

fn value(len: usize) -> String {
    let mut seed = 0x0741028_u64;
    (0..len)
        .map(|_| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            char::from(32 + (seed % 95) as u8)
        })
        .collect()
}

fn node() -> ChitchatId {
    ChitchatId::new("node".to_owned(), 1, "127.0.0.1:7280".parse().unwrap())
}

fn prepared(mtu: usize) -> DeltaSerializer {
    let mut s = DeltaSerializer::with_mtu(mtu);
    assert!(s.try_add_node(node(), 0, 0));
    s
}

fn op(text: &str) -> DeltaOp {
    DeltaOp::KeyValue(KeyValueMutation {
        key: "k".to_owned(),
        value: text.to_owned(),
        version: 1,
        status: DeletionStatusMutation::Set,
    })
}

fn complete(mtu: usize, text: &str) -> (bool, Vec<u8>) {
    let mut s = prepared(mtu);
    let accepted = s.try_add_kv("k", VersionedValue::for_test(text, 1));
    (accepted, s.finish().serialize_to_vec())
}

fn matrix() -> Vec<(usize, usize)> {
    let mut cases = Vec::new();
    for mtu in [100usize, 140, 198, 1000, 16384, 32768, 40000, 65507] {
        let mut lengths = vec![0, 1, 64, 4096, 16384, 32700, 65000];
        lengths.extend((48..=70).map(|n| mtu.saturating_sub(n)));
        lengths.sort_unstable();
        lengths.dedup();
        cases.extend(lengths.into_iter().map(|len| (mtu, len)));
    }
    assert_eq!(cases.len(), 239);
    cases
}

#[test]
fn allocator_control() {
    let (_, counts) = measured(|| unsafe {
        let a = Layout::from_size_align(128, 8).unwrap();
        let ptr = ALLOCATOR.alloc(a);
        assert!(!ptr.is_null());
        let ptr = ALLOCATOR.realloc(ptr, a, 256);
        assert!(!ptr.is_null());
        ALLOCATOR.dealloc(ptr, Layout::from_size_align(256, 8).unwrap());
        let ptr = ALLOCATOR.alloc_zeroed(a);
        assert!(!ptr.is_null());
        ALLOCATOR.dealloc(ptr, a);
    });
    assert_eq!(
        counts,
        Counts {
            allocs: 2,
            reallocs: 1,
            requested: 512,
            live: 0,
            peak: 256
        }
    );
    let (_, counts) = measured(|| black_box(1));
    assert_eq!(counts, Counts::default());
    assert!(std::panic::catch_unwind(|| measured(|| panic!("controlled"))).is_err());
    assert!(!ACTIVE.with(Cell::get));
}

#[test]
fn allocation_observations() {
    let mut rows = Vec::new();
    for (mtu, len) in matrix() {
        let text = value(len);
        let operation = op(&text);
        let s = prepared(mtu);
        let (buffers, threshold) = s.compressed_stream_writer.cost_inventory();
        let input = operation.serialized_len();
        let preview_branch = input <= u16::MAX as usize && buffers[1] + input > 2 * threshold;
        let (accepted, bytes) = complete(mtu, &text);
        let wire: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        let mut records = Vec::new();
        for _ in 0..2 {
            let (bound, bound_counts) = measured(|| {
                black_box(
                    s.compressed_stream_writer
                        .serialized_len_upperbound_after(black_box(&operation)),
                )
            });
            let (_, complete_counts) =
                measured(|| drop(black_box(complete(black_box(mtu), black_box(&text)))));
            assert_eq!(accepted, bound <= mtu);
            assert!(bytes.len() <= mtu);
            assert_eq!(bound_counts.live, 0);
            assert_eq!(complete_counts.live, 0);
            records.push(json!({"bound":bound,"bound_counts":bound_counts,"complete_counts":complete_counts}));
        }
        rows.push(json!({"mtu":mtu,"value_len":len,"accepted":accepted,"wire":wire,
            "source_preview_branch":preview_branch,"writer_lengths":buffers,
            "source_clone_bytes_if_preview":if preview_branch {buffers.iter().sum::<usize>()} else {0},
            "source_append_bytes_if_preview":if preview_branch {input} else {0},"records":records}));
    }
    println!(
        "HC074_PREVIEW_ALLOC:{}",
        json!({"seed":0x0741028_u64,"rows":rows,"native_allocations_included":false})
    );
}

#[cfg(windows)]
fn cpu() -> u64 {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentThread() -> *mut std::ffi::c_void;
        fn GetThreadTimes(
            thread: *mut std::ffi::c_void,
            created: *mut u64,
            exit: *mut u64,
            kernel: *mut u64,
            user: *mut u64,
        ) -> i32;
    }
    let (mut created, mut exit, mut kernel, mut user) = (0u64, 0u64, 0u64, 0u64);
    // SAFETY: current-thread pseudo handle is valid and four writable 8-byte
    // values have at least FILETIME's alignment. No ownership transfer.
    let ok = unsafe {
        GetThreadTimes(
            GetCurrentThread(),
            &mut created,
            &mut exit,
            &mut kernel,
            &mut user,
        )
    };
    assert_ne!(ok, 0);
    kernel + user
}

#[test]
#[cfg(windows)]
fn cpu_observations() {
    assert!(!ACTIVE.with(Cell::get));
    let mut rows = Vec::new();
    for (mtu, len, repetitions) in [
        (100, 1, 1024),
        (100, 4096, 64),
        (1000, 64, 1024),
        (1000, 4096, 64),
        (40000, 39938, 64),
        (65507, 65447, 64),
    ] {
        let text = value(len);
        let operation = op(&text);
        let s = prepared(mtu);
        black_box(
            s.compressed_stream_writer
                .serialized_len_upperbound_after(&operation),
        );
        drop(black_box(complete(mtu, &text)));
        let mut timings = Vec::new();
        for phase in ["bound", "complete"] {
            let before = cpu();
            let wall = Instant::now();
            for _ in 0..repetitions {
                if phase == "bound" {
                    black_box(
                        s.compressed_stream_writer
                            .serialized_len_upperbound_after(black_box(&operation)),
                    );
                } else {
                    drop(black_box(complete(black_box(mtu), black_box(&text))));
                }
            }
            let wall_ns = wall.elapsed().as_nanos();
            let elapsed_cpu_100ns = cpu()
                .checked_sub(before)
                .expect("thread CPU must not go backwards");
            timings.push(json!({"phase":phase,"cpu_100ns":elapsed_cpu_100ns,"wall_ns":wall_ns}));
        }
        rows.push(json!({"mtu":mtu,"value_len":len,"repetitions":repetitions,"timings":timings}));
    }
    println!(
        "HC074_PREVIEW_CPU:{}",
        json!({"seed":0x0741028_u64,"rows":rows,"tracking_enabled":false})
    );
}

// Actual production-call region; the old bound observer above remains a reference.
#[test]
fn operation_allocation_observations() {
    let mut rows = Vec::new();
    for (mtu, len) in matrix() {
        let text = value(len);
        let (accepted, bytes) = complete(mtu, &text);
        let wire: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        let mut records = Vec::new();
        for _ in 0..2 {
            crate::serialize::inplace_review::reset();
            let (_, counts) =
                measured(|| drop(black_box(complete(black_box(mtu), black_box(&text)))));
            let calls = crate::serialize::inplace_review::take();
            assert_eq!(counts.live, 0);
            records.push(json!({"complete_counts":counts,"actual_calls":calls}));
        }
        assert_eq!(records[0], records[1]);
        rows.push(
            json!({"mtu":mtu,"value_len":len,"accepted":accepted,"wire":wire,"records":records}),
        );
    }
    println!(
        "HC074_INPLACE_ALLOC:{}",
        json!({"seed":0x0741028_u64,"rows":rows,
        "actual_call_order":["reference_or_original_estimator","inplace_counter","old_budget_reject","conservative_accept"],
        "native_C_allocations_included":false})
    );
}
