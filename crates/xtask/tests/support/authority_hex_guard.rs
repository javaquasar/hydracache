//! Exact test hooks and one encoder substitution only; no body waivers.
#[macro_export]
macro_rules! hc074_authority_hex_view {
    ($path:expr, $source:expr) => {{
        let path: &str = $path;
        let mut raw: String = $source.to_owned();
        let gate = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/support/authority_hex_gate.txt")).replace('\r', "");
        for (file, exact, marker) in [
            ("tools/long-run-supervisor-074/src/diagnostic_authority_registry.rs", "\n#[cfg(all(test, target_os = \"linux\"))]\n#[path = \"../tests/support/authority_hex_controls.rs\"]\npub(crate) mod hex_controls;\n", "authority_hex_controls"),
            ("tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs", "\n#[path = \"../tests/support/authority_hex_alloc.rs\"]\nmod authority_hex_alloc;\n", "authority_hex_alloc"),
            ("docs/testing/gated-test-registry.toml", gate.as_str(), "# BEGIN AUTHORITY KEY HEX GATE"),
        ] {
            if path == file && raw.contains(marker) {
                assert_eq!(raw.matches(exact).count(), 1, "exact authority hex hook");
                raw = raw.replacen(exact, "", 1);
            }
        }
        if path == "tools/long-run-supervisor-074/src/diagnostic_authority_registry.rs" {
            let old = "fn key_hex(trust: &WorkerPolicyTrust) -> String {\n    trust\n        .key\n        .as_bytes()\n        .iter()\n        .map(|b| format!(\"{b:02x}\"))\n        .collect()\n}";
            let new = "fn key_hex(trust: &WorkerPolicyTrust) -> String {\n    crate::hex(trust.key.as_bytes())\n}";
            assert_eq!(raw.matches(old).count() + raw.matches(new).count(), 1, "one known authority encoder");
            raw = raw.replacen(new, old, 1);
        }
        raw
    }};
}
