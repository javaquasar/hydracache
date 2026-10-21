//! Remove only the exact opt-in test adapter, never a production predicate.
#[macro_export]
macro_rules! hc074_held_material_live_view {
    ($path:expr,$source:expr) => {{
        let path: &str = $path;
        let mut raw: String = $source.to_owned();
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
