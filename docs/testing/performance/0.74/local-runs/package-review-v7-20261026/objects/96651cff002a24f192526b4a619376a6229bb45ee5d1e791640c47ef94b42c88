//! A narrow reader successor; every legacy implementation and ledger stays unchanged.
#[path = "../../crates/xtask/tests/support/current_tree_input.rs"]
pub mod old;
pub(crate) use old::*;
#[path = "contract.rs"]
pub mod phase;

pub(crate) fn accepts_patch(path: &str, raw: &str) -> bool {
    if path == phase::FRESH {
        return phase::previous(path, raw)
            .is_some_and(|previous| old::accepts_patch(path, &previous));
    }
    old::accepts_patch(path, raw)
}
pub(crate) fn read(path: &str) -> String {
    if path != phase::FRESH {
        return old::read(path);
    }
    // Exactly the old patch branch's read and check order; no fallback or new IO.
    let raw = std::fs::read_to_string(crate::runtime::root().join(path)).unwrap();
    assert!(
        accepts_patch(path, &raw),
        "original fresh-kernel input gate after exact caller successor"
    );
    crate::runtime::frozen(SOURCE, path)
}
