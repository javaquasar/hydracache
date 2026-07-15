#[test]
fn miri_structural_contract_pins_toolchain_and_exact_evidence_gate() {
    let root = xtask::doc_check::find_repo_root().unwrap();
    xtask::miri_check::structural_check(&root).unwrap();
    assert_eq!(xtask::miri_check::PINNED_NIGHTLY, "nightly-2026-07-01");
}
