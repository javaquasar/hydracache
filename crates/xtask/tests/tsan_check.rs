#[test]
fn tsan_structural_contract_pins_toolchain_and_isolates_race_canary() {
    let root = xtask::doc_check::find_repo_root().unwrap();
    xtask::tsan_check::structural_check(&root).unwrap();
    assert_eq!(xtask::tsan_check::PINNED_NIGHTLY, "nightly-2026-07-01");

    let registry = xtask::gated_tests::load_registry(&root).unwrap();
    for id in [
        "tool.tsan.concurrent-suites",
        "tool.tsan.race-canary",
        "env.hydracache-require-tsan",
    ] {
        let gate = registry
            .gate
            .iter()
            .find(|gate| gate.id == id)
            .unwrap_or_else(|| panic!("missing TSan gate {id}"));
        assert_eq!(
            gate.timeout_seconds, 7200,
            "{id} must retain a bounded cold-build budget"
        );
    }
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
