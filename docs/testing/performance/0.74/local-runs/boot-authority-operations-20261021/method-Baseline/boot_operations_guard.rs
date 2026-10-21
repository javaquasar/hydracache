//! Only the exact opt-in operation hook and its manual gate may disappear.
#[macro_export]
macro_rules! hc074_boot_operations_view {
    ($path:expr, $source:expr) => {{
        let path: &str = $path;
        let mut raw: String = $source.to_owned();
        if path == "tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs"
            && raw.contains("let mut short_name = [0_u8; 24];")
        {
            let before = "let name = if !directory && name.len() < short_name.len() {";
            let after = "let name = if name.len() < short_name.len() {";
            assert_eq!(
                raw.matches(before).count() + raw.matches(after).count(),
                1,
                "exact directory name ownership predicate"
            );
            if raw.contains(after) {
                raw = raw.replacen(after, before, 1);
            }
        }
        let gate = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/support/boot_operations_gate.txt"
        ))
        .replace('\r', "");
        for (file, exact, marker) in [
            (
                "tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs",
                "\n#[path = \"../tests/support/boot_operations.rs\"]\nmod boot_operations;\n",
                "boot_operations",
            ),
            (
                "docs/testing/gated-test-registry.toml",
                format!("\n{gate}").as_str(),
                "boot_operations",
            ),
        ] {
            if path == file && raw.contains(marker) {
                assert_eq!(raw.matches(exact).count(), 1, "exact Boot operation hook");
                raw = raw.replacen(exact, "", 1);
            }
        }
        raw
    }};
}
