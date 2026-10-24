//! Remove only the exact opt-in test adapter, never a production predicate.
#[path = "pidfd_info_path_guard.rs"]
mod pidfd;
#[macro_export]
macro_rules! hc074_held_material_live_view {
    ($path:expr,$source:expr) => {{
        let path: &str = $path;
        let mut raw: String = $crate::hc074_pidfd_info_path_view!(path, $source);
        for (file, hook) in [
            ("tools/long-run-supervisor-074/tests/support/consuming_held_live.rs", "#[path = \"held_original_service_live.rs\"]\nmod original_service;\n"),
            ("tools/long-run-supervisor-074/tests/support/consuming_held_live_documents.rs", "#[path = \"held_original_service_documents.rs\"]\nmod original_service;\n"),
        ] {
            if path == file && raw.contains("mod original_service;") {
                assert_eq!(raw.matches(hook).count(), 1);
                raw = raw.replacen(hook, "", 1);
            }
        }
        if path == "docs/testing/gated-test-registry.toml" && raw.contains("# BEGIN ORIGINAL MANAGER SERVICE GATE") {
            let gate = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/support/original_service_gate.txt")).replace('\r', "");
            assert_eq!(raw.matches(gate.as_str()).count(), 1);
            raw = raw.replacen(gate.as_str(), "", 1);
        }
        if path == "tools/long-run-supervisor-074/src/diagnostic_held_unit_material.rs"
            && raw.contains("held_material_live_api.rs")
        {
            let hook = "\n#[cfg(test)]\n#[path = \"../tests/support/held_material_live_api.rs\"]\nmod live_api;\n";
            assert_eq!(raw.matches(hook).count(), 1);
            raw = raw.replacen(hook, "", 1);
        }
        if path == "tools/long-run-supervisor-074/tests/support/consuming_held_live.rs"
            && raw.contains("HC074_HELD_MATERIAL_LIVE")
        {
            let hook = "                if let Ok(mode) = std::env::var(\"HC074_HELD_MATERIAL_LIVE\") {\n                    assert_eq!(mode, \"original-once-v1\");\n                    assert_eq!(case, \"healthy\");\n                    measured(\"projection-materialization\", || held.fixture_materialize());\n                } else {\n                    drop(held);\n                }";
            assert_eq!(raw.matches(hook).count(), 1);
            raw = raw.replacen(hook, "                drop(held);", 1);
        }
        raw
    }};
}
