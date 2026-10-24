//! One exact test-source comparison edit; no IO, cache or validation bypass.
pub const SOURCE: &str = "crates/xtask/tests/support/performance_contract_074_fixed_slice.rs";
pub const OLD: &str = "if path == format!(\"{PREFIX}{file}\") {";
pub const NEW: &str = "if path.strip_prefix(PREFIX) == Some(*file) {";

pub fn candidate(original: &str) -> String {
    assert_eq!(original.matches(OLD).count(), 1, "exact path comparison");
    assert_eq!(
        original.matches(NEW).count(),
        0,
        "already changed comparison"
    );
    original.replacen(OLD, NEW, 1)
}
