//! Exact additive materialization hooks; no old predicate/body exclusion.
#[path = "held_material_live_guard.rs"]
mod live;
#[macro_export]
macro_rules! hc074_held_material_view {
    ($path:expr,$source:expr) => {{
        let path: &str = $path;
        let mut raw: String = $crate::hc074_held_material_live_view!(path, $source);
        for (file, child) in [
            (
                "diagnostic_held_unit_projection.rs",
                "diagnostic_held_unit_material.rs",
            ),
            (
                "diagnostic_bundle_projection.rs",
                "diagnostic_bundle_material.rs",
            ),
        ] {
            if path == format!("tools/long-run-supervisor-074/src/{file}") && raw.contains(child) {
                let exact = format!("\n#[path = \"{child}\"]\nmod material;\n");
                assert_eq!(
                    raw.matches(&exact).count(),
                    1,
                    "exact material hook: {path}"
                );
                raw = raw.replacen(&exact, "", 1);
            }
        }
        raw
    }};
}
