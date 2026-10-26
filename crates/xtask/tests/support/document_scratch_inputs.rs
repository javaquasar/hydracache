//! Exact current reader bytes before a preceding test phase; no runtime trust.
pub(crate) const REVIEWED: &str = "b0e7fcac2ef33974df1633ada55457478c624498";
pub(crate) const PREDECESSOR: &str = "33c54b0cda8c330b115009cdf092ad002077cedf";
pub(crate) const PATHS: [&str; 5] = [
    "crates/xtask/tests/support/performance_contract_074_kernel_walk.rs",
    "crates/xtask/tests/support/performance_contract_074_boot_comparison.rs",
    "crates/xtask/tests/fresh_kernel_cost_074.rs",
    "crates/xtask/tests/pid_path_cost_074.rs",
    "crates/xtask/tests/held_preparation_cost_074.rs",
];
fn accepts_previous(path: &str, raw: &str) -> bool {
    PATHS.contains(&path) && raw.replace("\r\n", "\n") == crate::runtime::frozen(REVIEWED, path)
}
pub(crate) fn predecessor(path: &str, raw: &str) -> String {
    assert!(
        accepts(path, raw),
        "unreviewed document scratch reader: {path}"
    );
    crate::runtime::frozen(PREDECESSOR, path)
}
pub(crate) fn read(path: &str) -> String {
    let input = crate::runtime::root().join(path);
    if PATHS.contains(&path) {
        return predecessor(path, &std::fs::read_to_string(input).unwrap());
    }
    crate::runtime_io::read_to_string(input)
        .unwrap()
        .replace('\r', "")
}

#[path = "../../../../tools/remaining-readers-074/contract.rs"]
mod remaining_phase;
pub(crate) fn accepts(path: &str, raw: &str) -> bool {
    if path == remaining_phase::FRESH {
        return remaining_phase::previous(path, raw)
            .is_some_and(|previous| accepts_previous(path, &previous));
    }
    accepts_previous(path, raw)
}
