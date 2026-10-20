//! Exact expected-name ownership substitution only; unrelated drift is retained.
#[macro_export]
macro_rules! hc074_authority_expected_name_view {
    ($path:expr, $source:expr) => {{
        let path: &str = $path;
        let mut raw: String = $source.to_owned();
        let gate = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/support/authority_expected_name_gate.txt")).replace('\r', "");
        for (file, exact, marker) in [
            ("tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs", "\n#[cfg(test)]\n#[path = \"../tests/support/authority_expected_name_controls.rs\"]\npub(crate) mod expected_name_controls;\n", "authority_expected_name_controls"),
            ("tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs", "\n#[path = \"../tests/support/authority_expected_name_alloc.rs\"]\nmod authority_expected_name_alloc;\n", "authority_expected_name_alloc"),
            ("docs/testing/gated-test-registry.toml", gate.as_str(), "# BEGIN AUTHORITY EXPECTED NAME GATE"),
        ] {
            if path == file && raw.contains(marker) {
                assert_eq!(raw.matches(exact).count(), 1, "exact expected-name hook");
                raw = raw.replacen(exact, "", 1);
            }
        }
        if path == "tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs" {
            let old = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/support/authority_expected_name_old.txt")).replace('\r', "");
            let new = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/support/authority_expected_name_new.txt")).replace('\r', "");
            let old = old.trim_end_matches('\n');
            let new = new.trim_end_matches('\n');
            let legacy = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/support/authority_inventory_old.txt")).replace('\r', "");
            let legacy = legacy.trim_end_matches('\n');
            assert_eq!(raw.matches(old).count() + raw.matches(new).count() + raw.matches(legacy).count(), 1, "one known complete inventory");
            raw = raw.replacen(new, old, 1);
            assert!(!raw.contains("fn expected_inventory"), "unknown expected-name helper");
        }
        raw
    }};
}
