//! Strip only the exact selected-CPU test hook and manual registration.
#[path = "authority_open_name_guard.rs"]
mod authority_open_name;
#[macro_export]
macro_rules! hc074_authority_leaf_cpu_view {
    ($path:expr, $source:expr) => {{
        let path: &str = $path;
        let mut raw: String = $crate::hc074_authority_open_name_view!(path, $source);
        let gate = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/support/authority_leaf_cpu_gate.txt"
        ))
        .replace('\r', "");
        for (file, exact, marker) in [
            (
                "tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs",
                "\n#[path = \"../tests/support/authority_leaf_cpu.rs\"]\nmod authority_leaf_cpu;\n",
                "authority_leaf_cpu",
            ),
            (
                "docs/testing/gated-test-registry.toml",
                gate.as_str(),
                "# BEGIN AUTHORITY LEAF CPU GATE",
            ),
        ] {
            if path == file && raw.contains(marker) {
                assert_eq!(raw.matches(exact).count(), 1, "exact leaf CPU hook");
                raw = raw.replacen(exact, "", 1);
            }
        }
        raw
    }};
}
