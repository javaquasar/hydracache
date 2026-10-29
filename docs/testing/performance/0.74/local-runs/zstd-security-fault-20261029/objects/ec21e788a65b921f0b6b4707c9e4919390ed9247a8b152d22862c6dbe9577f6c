//! Verify the sole extended proof reader before exposing its previous phase.
#[path = "contract.rs"]
pub mod contract;
pub fn read(path: &str, raw: String) -> String {
    if path == "crates/xtask/tests/current_tree_reader_074.rs" {
        return remaining_phase::previous(path, &raw)
            .expect("exact next current-tree reader phase");
    }
    if path != "crates/xtask/tests/owned_case_074.rs" {
        return raw;
    }
    let old = crate::runtime::frozen("181c077abdfbae5a8c4dfe67cee6f1c95519d529", path);
    assert_eq!(
        raw.replace("\r\n", "\n"),
        contract::adapt(path, &old).unwrap(),
        "unreviewed owned-case proof reader"
    );
    old
}

#[path = "../remaining-readers-074/contract.rs"]
mod remaining_phase;
