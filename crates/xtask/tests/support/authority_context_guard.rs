//! Exact context binding substitution only, preserving the accepted key encoder.
#[macro_export]
macro_rules! hc074_authority_context_view {
    ($path:expr, $source:expr) => {{
        let path: &str = $path;
        let mut raw: String = $source.to_owned();
        let gate = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/support/authority_context_gate.txt")).replace('\r', "");
        for (file, exact, marker) in [
            ("tools/long-run-supervisor-074/src/diagnostic_authority_registry.rs", "\n#[cfg(all(test, target_os = \"linux\"))]\n#[path = \"../tests/support/authority_context_controls.rs\"]\npub(crate) mod context_controls;\n", "authority_context_controls"),
            ("tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs", "\n#[path = \"../tests/support/authority_context_alloc.rs\"]\nmod authority_context_alloc;\n", "authority_context_alloc"),
            ("docs/testing/gated-test-registry.toml", gate.as_str(), "# BEGIN AUTHORITY CONTEXT GATE"),
        ] {
            if path == file && raw.contains(marker) {
                assert_eq!(raw.matches(exact).count(), 1, "exact authority context hook");
                raw = raw.replacen(exact, "", 1);
            }
        }
        if path == "tools/long-run-supervisor-074/src/diagnostic_authority_registry.rs" {
            let old = "            || self.context() != *host\n";
            let new = "            || self.machine_id != host.machine_id\n            || self.boot_id != host.boot_id\n            || self.user_namespace != host.user_namespace\n            || self.mount_namespace != host.mount_namespace\n";
            assert_eq!(raw.matches(old).count() + raw.matches(new).count(), 1, "one known context binding");
            raw = raw.replacen(new, old, 1);
        }
        raw
    }};
}
