//! Diagnostic only. No allocations by the observer inside its measured region.
use super::*;
use std::alloc::{GlobalAlloc, Layout, System};
const SOURCE_VARIANT: &str = "B0";
const EXPECTED_CONTEXT: bool = false;

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize)]
struct Stats {
    malloc_calls: u64,
    calloc_calls: u64,
    realloc_calls: u64,
    free_calls: u64,
    allocs: u64,
    requested: u64,
    failed: u64,
    external_frees: u64,
    live: i64,
    peak: i64,
}
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize)]
struct Snapshot {
    c: Stats,
    rust: Stats,
    errors: u64,
}
unsafe extern "C" {
    fn hc_begin() -> i32;
    fn hc_end(out: *mut Snapshot) -> i32;
    fn hc_peek(out: *mut Snapshot) -> i32;
    fn hc_rust_enter();
    fn hc_rust_leave();
    fn hc_c_probe() -> i32;
}
fn tagged<T>(operation: impl FnOnce() -> T) -> T {
    // SAFETY: allocation-free TLS depth tag, balanced including unwind.
    unsafe { hc_rust_enter() };
    struct Leave;
    impl Drop for Leave {
        fn drop(&mut self) {
            // SAFETY: exactly balances this tag's entry.
            unsafe { hc_rust_leave() };
        }
    }
    let leave = Leave;
    let value = operation();
    drop(leave);
    value
}
struct Meter;
// SAFETY: every pointer, layout and alignment delegates unchanged to System.
// Tags only identify Rust calls to the native observer; they allocate nothing.
unsafe impl GlobalAlloc for Meter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        tagged(|| unsafe { System.alloc(layout) })
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        tagged(|| unsafe { System.alloc_zeroed(layout) })
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        tagged(|| unsafe { System.dealloc(ptr, layout) });
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        tagged(|| unsafe { System.realloc(ptr, layout, size) })
    }
}
#[global_allocator]
static METER: Meter = Meter;
fn begin() {
    assert_eq!(std::mem::size_of::<Stats>(), 80);
    assert_eq!(std::mem::size_of::<Snapshot>(), 168);
    // SAFETY: no previous active region on this test thread.
    assert_eq!(unsafe { hc_begin() }, 1);
}
fn snapshot(end: bool) -> Snapshot {
    let mut value = Snapshot::default();
    // SAFETY: valid exclusive repr(C) pointer for this synchronous call only.
    assert_eq!(
        unsafe {
            if end {
                hc_end(&mut value)
            } else {
                hc_peek(&mut value)
            }
        },
        1
    );
    assert_eq!(value.errors, 0);
    for stats in [value.c, value.rust] {
        assert_eq!(stats.failed, 0);
        assert_eq!(stats.external_frees, 0);
        assert!(stats.live >= 0);
        if end {
            assert_eq!(stats.live, 0);
        }
    }
    value
}
#[test]
fn allocator_controls() {
    begin();
    // SAFETY: original C probe owns and frees every successful allocation.
    assert_eq!(unsafe { hc_c_probe() }, 1);
    let c = snapshot(true);
    assert_eq!(
        (c.c.allocs, c.c.realloc_calls, c.c.requested, c.c.peak),
        (2, 1, 512, 256)
    );
    assert_eq!(c.rust, Stats::default());
    begin();
    // SAFETY: System pointers are resized/freed with their exact current layout.
    unsafe {
        let layout = Layout::from_size_align(128, 8).unwrap();
        let p = METER.alloc(layout);
        assert!(!p.is_null());
        std::ptr::write_volatile(p, 0x71);
        let p = METER.realloc(p, layout, 256);
        assert!(!p.is_null());
        assert_eq!(std::ptr::read_volatile(p), 0x71);
        METER.dealloc(p, Layout::from_size_align(256, 8).unwrap());
        let p = METER.alloc_zeroed(layout);
        assert!(!p.is_null());
        assert_eq!(std::ptr::read_volatile(p), 0);
        METER.dealloc(p, layout);
    }
    let rust = snapshot(true);
    assert_eq!(
        (
            rust.rust.allocs,
            rust.rust.realloc_calls,
            rust.rust.requested,
            rust.rust.peak
        ),
        (2, 1, 512, 256)
    );
    assert_eq!(rust.c, Stats::default());
}
#[derive(Debug, Clone, Copy, serde::Serialize)]
struct Inventory {
    len: [usize; 3],
    capacity: [usize; 3],
}
fn inventory(w: &CompressedStreamWriter) -> Inventory {
    Inventory {
        len: [
            w.output.len(),
            w.uncompressed_block.len(),
            w.compressed_block.len(),
        ],
        capacity: [
            w.output.capacity(),
            w.uncompressed_block.capacity(),
            w.compressed_block.capacity(),
        ],
    }
}
#[test]
fn context_lifetime_observation() {
    let first = "first job ".repeat(30);
    let second = "second job ".repeat(30);
    begin();
    let mut writer = CompressedStreamWriter::with_block_threshold(64);
    let construct = (snapshot(false), inventory(&writer));
    let constructed_context = writer.compression_context.is_some();
    writer.append(first.as_str());
    let append1 = (snapshot(false), inventory(&writer));
    let append1_context = writer.compression_context.is_some();
    writer.append(second.as_str());
    let append2 = (snapshot(false), inventory(&writer));
    let append2_context = writer.compression_context.is_some();
    let bytes = writer.finish();
    let finish = snapshot(false);
    let mut wire = [0u8; 1024];
    assert!(bytes.len() <= wire.len());
    let wire_len = bytes.len();
    wire[..wire_len].copy_from_slice(&bytes);
    drop(bytes);
    let retirement = snapshot(true);
    let context_present = [constructed_context, append1_context, append2_context];
    assert_eq!(context_present, [false, EXPECTED_CONTEXT, EXPECTED_CONTEXT]);
    let mut input = &wire[..wire_len];
    let decoded: Vec<String> = deserialize_stream(&mut input).unwrap();
    assert_eq!(decoded, [first, second]);
    println!(
        "HC074_CONTEXT_LIFETIME:{}",
        serde_json::json!({
            "threshold":64,"repetitions":30,"construct":construct,"append1":append1,
        "variant":SOURCE_VARIANT,"context_present":context_present,
            "append2":append2,"finish":finish,"retirement":retirement,
            "wire":wire[..wire_len],"CPU_qualified":false,"RSS_qualified":false,
            "allocator_page_retention_qualified":false
        })
    );
}
