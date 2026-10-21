//! Exact path ownership and test hooks only; preserve all unrelated drift.
#[path = "authority_owners_guard.rs"]
mod authority_owners;
#[macro_export]
macro_rules! hc074_authority_fd_path_view {
    ($path:expr, $source:expr) => {{
        let path: &str = $path;
        let mut raw: String = $crate::hc074_authority_owners_view!(path, $source);
        let gate = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/support/authority_fd_path_gate.txt")).replace('\r', "");
        for (file, exact, marker) in [
            ("tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs", "\n#[cfg(test)]\n#[path = \"../tests/support/authority_fd_path_controls.rs\"]\npub(crate) mod fd_path_controls;\n", "authority_fd_path_controls"),
            ("tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs", "\n#[path = \"../tests/support/authority_fd_path_alloc.rs\"]\nmod authority_fd_path_alloc;\n", "authority_fd_path_alloc"),
            ("docs/testing/gated-test-registry.toml", gate.as_str(), "# BEGIN AUTHORITY FD PATH GATE"),
        ] {
            if path == file && raw.contains(marker) {
                assert_eq!(raw.matches(exact).count(), 1, "exact fd-path hook");
                raw = raw.replacen(exact, "", 1);
            }
        }
        if path == "tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs" {
            let old = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/support/authority_fd_path_old.txt")).replace('\r', "");
            let new = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/support/authority_fd_path_new.txt")).replace('\r', "");
            let old = old.trim_end_matches('\n');
            let new = new.trim_end_matches('\n');
            assert_eq!(raw.matches(old).count() + raw.matches(new).count(), 1, "one known fd-path construction");
            raw = raw.replacen(new, old, 1);
        }
        raw
    }};
}
