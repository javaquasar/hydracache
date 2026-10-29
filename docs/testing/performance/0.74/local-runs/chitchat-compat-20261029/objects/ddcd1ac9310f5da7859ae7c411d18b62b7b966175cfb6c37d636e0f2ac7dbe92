// Appended to the unchanged portable observer; only System calls gain Rust tags.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize)]
struct NativeStats {
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
struct NativeSnapshot {
    c: NativeStats,
    rust: NativeStats,
    errors: u64,
}
unsafe extern "C" {
    fn hc_begin() -> i32;
    fn hc_end(out: *mut NativeSnapshot) -> i32;
    fn hc_rust_enter();
    fn hc_rust_leave();
    fn hc_c_probe() -> i32;
}
fn tag_system<T>(operation: impl FnOnce() -> T) -> T {
    // SAFETY: diagnostic TLS tag has no pointers and allocates no memory.
    unsafe { hc_rust_enter() };
    struct Leave;
    impl Drop for Leave {
        fn drop(&mut self) {
            // SAFETY: exactly balances this call's entry, including unwind.
            unsafe { hc_rust_leave() };
        }
    }
    let leave = Leave;
    let result = operation();
    drop(leave);
    result
}
fn native_begin() {
    assert_eq!(std::mem::size_of::<NativeStats>(), 80);
    assert_eq!(std::mem::size_of::<NativeSnapshot>(), 168);
    // SAFETY: linked diagnostic object has no retained Rust references. Its
    // fixed thread-local state is inactive and both images use the same ABI.
    assert_eq!(unsafe { hc_begin() }, 1);
}
fn native_finish() -> NativeSnapshot {
    let mut result = NativeSnapshot::default();
    // SAFETY: valid exclusive pointer to the repr(C) output for this call only.
    assert_eq!(unsafe { hc_end(&mut result) }, 1);
    assert_eq!(
        result.errors, 0,
        "counter overflow, table or ownership mismatch"
    );
    for stats in [result.c, result.rust] {
        assert_eq!(stats.live, 0, "whole-operation allocation balance");
        assert_eq!(stats.external_frees, 0, "out-of-region pointer refused");
        assert_eq!(stats.failed, 0, "allocator failure refuses observation");
    }
    result
}
#[test]
fn native_c_allocator_control() {
    native_begin();
    // SAFETY: owned C probe has no arguments and frees all successful allocations.
    assert_eq!(unsafe { hc_c_probe() }, 1);
    let result = native_finish();
    assert_eq!(result.rust, NativeStats::default());
    assert_eq!(
        (
            result.c.allocs,
            result.c.realloc_calls,
            result.c.requested,
            result.c.peak
        ),
        (2, 1, 512, 256)
    );
}
#[test]
fn native_rust_tag_control() {
    native_begin();
    let (_, rust) = measured(|| {
        // SAFETY: successful System allocation is resized/deallocated with its
        // exact current layout. This region excludes the legacy panic control.
        unsafe {
            let layout = Layout::from_size_align(128, 8).unwrap();
            let p = black_box(ALLOCATOR.alloc(layout));
            assert!(!p.is_null());
            std::ptr::write_volatile(p, 0x7b);
            let q = black_box(ALLOCATOR.realloc(black_box(p), layout, 256));
            assert!(!q.is_null());
            assert_eq!(std::ptr::read_volatile(q), 0x7b);
            ALLOCATOR.dealloc(black_box(q), Layout::from_size_align(256, 8).unwrap());
            let p = black_box(ALLOCATOR.alloc_zeroed(layout));
            assert!(!p.is_null());
            assert_eq!(std::ptr::read_volatile(p), 0);
            ALLOCATOR.dealloc(black_box(p), layout);
        }
    });
    let result = native_finish();
    assert_eq!(
        (
            rust.allocs,
            rust.reallocs,
            rust.requested,
            rust.live,
            rust.peak
        ),
        (2, 1, 512, 0, 256)
    );
    assert_eq!(result.c, NativeStats::default());
    assert_eq!(
        (
            result.rust.allocs,
            result.rust.realloc_calls,
            result.rust.requested,
            result.rust.peak
        ),
        (2, 1, 512, 256)
    );
}
#[test]
fn native_operation_observations() {
    let mut rows = Vec::new();
    for (mtu, len) in matrix() {
        let text = value(len);
        let (accepted, bytes) = complete(mtu, &text);
        let wire: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        let mut records = Vec::new();
        for _ in 0..2 {
            crate::serialize::inplace_review::reset();
            native_begin();
            let (_, rust) =
                measured(|| drop(black_box(complete(black_box(mtu), black_box(&text)))));
            let native = native_finish();
            assert_eq!(rust.live, 0);
            let actual_calls = crate::serialize::inplace_review::take();
            records.push(json!({"rust":rust,"native":native,"actual_calls":actual_calls}));
        }
        assert_eq!(records[0], records[1]);
        rows.push(
            json!({"mtu":mtu,"value_len":len,"accepted":accepted,"wire":wire,"records":records}),
        );
    }
    println!(
        "HC074_NATIVE_ALLOC:{}",
        json!({"seed":0x0741028_u64,"rows":rows,
        "scope":"linked-default-C-malloc-family; Rust System explicitly tagged",
        "CPU_qualified":false,"allocator_retention_qualified":false})
    );
}
