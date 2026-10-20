//! Exact name capacity view and old input-helper restoration only.
#[macro_export]
macro_rules! hc074_authority_name_capacity_view {
    ($path:expr, $source:expr) => {{
        let path: &str = $path;
        let mut raw: String = $source.to_owned();
        let gate = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/support/authority_name_capacity_gate.txt")).replace('\r', "");
        for (file, exact, marker) in [
            ("tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs", "\n#[cfg(test)]\n#[path = \"../tests/support/authority_name_capacity_controls.rs\"]\npub(crate) mod name_capacity_controls;\n", "authority_name_capacity_controls"),
            ("tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs", "\n#[path = \"../tests/support/authority_name_capacity_alloc.rs\"]\nmod authority_name_capacity_alloc;\n", "authority_name_capacity_alloc"),
            ("docs/testing/gated-test-registry.toml", gate.as_str(), "# BEGIN AUTHORITY NAME CAPACITY GATE"),
        ] {
            if path == file && raw.contains(marker) {
                assert_eq!(raw.matches(exact).count(), 1, "exact name capacity hook");
                raw = raw.replacen(exact, "", 1);
            }
        }
        for (file, old, new) in [
            ("tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs",
             include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/support/authority_name_capacity_old.txt")),
             include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/support/authority_name_capacity_new.txt"))),
            ("crates/xtask/tests/authority_inventory_074.rs",
             include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/support/authority_name_capacity_read_old.txt")),
             include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/support/authority_name_capacity_read_new.txt"))),
        ] {
            if path == file {
                let old = old.replace('\r', "");
                let new = new.replace('\r', "");
                let old = old.trim_end_matches('\n');
                let new = new.trim_end_matches('\n');
                assert_eq!(raw.matches(old).count() + raw.matches(new).count(), 1, "one known name or input helper");
                raw = raw.replacen(new, old, 1);
            }
        }
        raw
    }};
}
