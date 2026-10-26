//! Attribution hooks only; no runtime substitution.
#[path = "authority_cpu_guard.rs"]
mod authority_cpu;
#[macro_export]
macro_rules! hc074_authority_owners_view {
    ($path:expr, $source:expr) => {{
        let path: &str = $path;
        let mut raw: String = $crate::hc074_authority_cpu_view!(path, $source);
        let gate = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/support/authority_owners_gate.txt")).replace('\r', "");
        for (file, exact, marker) in [
            ("tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs", "\n#[cfg(test)]\n#[path = \"../tests/support/authority_owners_controls.rs\"]\npub(crate) mod owners_controls;\n", "authority_owners_controls"),
            ("tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs", "\n#[path = \"../tests/support/authority_owners_alloc.rs\"]\nmod authority_owners_alloc;\n", "authority_owners_alloc"),
            ("docs/testing/gated-test-registry.toml", gate.as_str(), "# BEGIN AUTHORITY OWNERS GATE"),
        ] {
            if path == file && raw.contains(marker) {
                assert_eq!(raw.matches(exact).count(), 1, "exact owners hook");
                raw = raw.replacen(exact, "", 1);
            }
        }
        raw
    }};
}
