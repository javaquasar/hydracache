//! Restore only the exact bounded file-name allocation and exact test hooks.
#[macro_export]
macro_rules! hc074_authority_open_name_view {
    ($path:expr, $source:expr) => {{
        let path: &str = $path;
        let mut raw: String = $source.to_owned();
        if path == "tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs"
            && raw.contains("let mut short_name = [0_u8; 24];")
        {
            let candidate = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/support/authority_open_name_candidate.txt")).replace('\r', "");
            assert_eq!(raw.matches(candidate.as_str()).count(), 1, "exact short file-name allocation");
            raw = raw.replacen(candidate.as_str(), "fn open_at(parent: &File, name: &str, directory: bool) -> Result<File, AuthorityError> {\n    let name = CString::new(name).map_err(|_| AuthorityError::Invalid)?;\n", 1);
        }
        let gate = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/support/authority_open_name_gate.txt")).replace('\r', "");
        for (file, exact, marker) in [
            ("tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs", "\n#[cfg(test)]\n#[path = \"../tests/support/authority_open_name_controls.rs\"]\npub(crate) mod open_name_controls;\n", "open_name_controls"),
            ("tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs", "\n#[path = \"../tests/support/authority_open_name_alloc.rs\"]\nmod authority_open_name_alloc;\n", "authority_open_name_alloc"),
            ("docs/testing/gated-test-registry.toml", gate.as_str(), "# BEGIN AUTHORITY OPEN NAME GATE"),
        ] {
            if path == file && raw.contains(marker) {
                assert_eq!(raw.matches(exact).count(), 1, "exact open-name hook");
                raw = raw.replacen(exact, "", 1);
            }
        }
        raw
    }};
}
