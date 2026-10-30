// Diagnostic only: qualify hc_peek and check C ownership after actual calls.
thread_local! {static C_FREE_CHECKS:Cell<u64>=const{Cell::new(0)};}
unsafe extern "C" {
    fn hc_peek(out: *mut NativeSnapshot) -> i32;
    fn malloc(size: usize) -> *mut std::ffi::c_void;
    fn free(pointer: *mut std::ffi::c_void);
}
pub(crate) fn c_free_checkpoint() {
    if !ACTIVE.with(Cell::get) {
        return;
    }
    let mut value = NativeSnapshot::default();
    // SAFETY: synchronous call writes one exclusive repr(C) snapshot. The C
    // observer is already active; no Rust pointer/layout or ownership changes.
    assert_eq!(unsafe { hc_peek(&mut value) }, 1);
    assert_eq!(value.errors, 0);
    assert_eq!(value.c.failed, 0);
    assert_eq!(value.c.external_frees, 0);
    assert_eq!(value.c.live, 0, "retained owned C bytes");
    assert_eq!(
        value.rust.live,
        COUNTS.with(Cell::get).live,
        "logical/physical Rust live"
    );
    C_FREE_CHECKS.with(|v| v.set(v.get() + 1));
}
fn c_free_checks() -> u64 {
    C_FREE_CHECKS.with(Cell::get)
}

#[test]
fn c_checkpoint_positive_and_retained_negative_control() {
    c_free_checkpoint(); // Inactive observer must stay inert.
    native_begin();
    let (_, logical) = measured(|| {
        c_free_checkpoint();
        // SAFETY: this C-owned allocation is checked and freed exactly once in
        // the same active region, including after the deliberately caught fault.
        let pointer = unsafe { malloc(64) };
        assert!(!pointer.is_null());
        let refused = std::panic::catch_unwind(std::panic::AssertUnwindSafe(c_free_checkpoint));
        assert!(refused.is_err());
        drop(refused);
        unsafe { free(pointer) };
        c_free_checkpoint();
    });
    assert_eq!(logical.live, 0);
    let physical = native_finish();
    assert_eq!(physical.c.live, 0);
    assert_eq!(c_free_checks(), 2);
}
