//! Remove only the exact opt-in test hook and manual registration.
#[path = "authority_leaf_cpu_guard.rs"]
mod authority_leaf_cpu;
#[macro_export]
macro_rules! hc074_authority_cpu_view {
    ($path:expr, $source:expr) => {{
        let path: &str = $path;
        let mut raw: String = $crate::hc074_authority_leaf_cpu_view!(path, $source);
        let gate = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/support/authority_cpu_gate.txt"
        ))
        .replace('\r', "");
        for (file, exact, marker) in [
            (
                "tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs",
                "\n#[path = \"../tests/support/authority_cpu.rs\"]\nmod authority_cpu;\n",
                "authority_cpu",
            ),
            (
                "docs/testing/gated-test-registry.toml",
                gate.as_str(),
                "# BEGIN AUTHORITY CPU GATE",
            ),
        ] {
            if path == file && raw.contains(marker) {
                assert_eq!(raw.matches(exact).count(), 1, "exact CPU hook");
                raw = raw.replacen(exact, "", 1);
            }
        }
        raw
    }};
}
