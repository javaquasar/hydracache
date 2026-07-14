#[test]
fn tsan_structural_contract_pins_toolchain_and_isolates_race_canary() {
    let root = xtask::doc_check::find_repo_root().unwrap();
    xtask::tsan_check::structural_check(&root).unwrap();
    assert_eq!(xtask::tsan_check::PINNED_NIGHTLY, "nightly-2026-07-01");
}

#[test]
fn tsan_red_canary_evidence_requires_nonzero_and_normalized_race_signature() {
    assert!(xtask::tsan_check::canary_output_is_expected_red(
        false,
        "WARNING: ThreadSanitizer: data race"
    ));
    assert!(!xtask::tsan_check::canary_output_is_expected_red(
        true,
        "WARNING: ThreadSanitizer: data race"
    ));
    assert!(!xtask::tsan_check::canary_output_is_expected_red(
        false,
        "test panicked for an unrelated reason"
    ));
}
