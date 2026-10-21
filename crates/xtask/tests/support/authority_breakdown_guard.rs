//! Test-only hooks; no runtime body substitution.
#[macro_export]
macro_rules! hc074_authority_breakdown_view {
    ($path:expr, $source:expr) => {{
        let path: &str = $path;
        let mut raw: String = $source.to_owned();
        let gate = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/support/authority_breakdown_gate.txt")).replace('\r', "");
        for (file, exact, marker) in [
            ("tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs", "\n#[cfg(test)]\n#[path = \"../tests/support/authority_breakdown_controls.rs\"]\npub(crate) mod breakdown_controls;\n", "authority_breakdown_controls"),
            ("tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs", "\n#[path = \"../tests/support/authority_breakdown_alloc.rs\"]\nmod authority_breakdown_alloc;\n", "authority_breakdown_alloc"),
            ("docs/testing/gated-test-registry.toml", gate.as_str(), "# BEGIN AUTHORITY BREAKDOWN GATE"),
        ] {
            if path == file && raw.contains(marker) {
                assert_eq!(raw.matches(exact).count(), 1, "exact breakdown hook");
                raw = raw.replacen(exact, "", 1);
            }
        }
        raw
    }};
}
