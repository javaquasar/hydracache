//! Exact additive projection hooks only. No validation body may disappear.
#[path = "held_material_guard.rs"]
mod material;
#[macro_export]
macro_rules! hc074_held_unit_projection_view {
    ($path:expr, $source:expr) => {{
        let path: &str = $path;
        let mut raw: String = $crate::hc074_held_material_view!(path, $source);
        for (file, hook) in [
            (
                "diagnostic_consuming_held_join.rs",
                "diagnostic_held_unit_projection.rs",
            ),
            (
                "diagnostic_held_role_files.rs",
                "diagnostic_held_role_projection.rs",
            ),
            (
                "diagnostic_artifacts_linux.rs",
                "diagnostic_bundle_projection.rs",
            ),
            (
                "diagnostic_root_guardian_original_kernel.rs",
                "diagnostic_root_guardian_projection.rs",
            ),
        ] {
            if path == format!("tools/long-run-supervisor-074/src/{file}") && raw.contains(hook) {
                let module = if file == "diagnostic_consuming_held_join.rs" {
                    "unit_projection"
                } else {
                    "projection"
                };
                let exact = format!("\n#[path = \"{hook}\"]\nmod {module};\n");
                assert_eq!(
                    raw.matches(&exact).count(),
                    1,
                    "exact held projection hook: {path}"
                );
                raw = raw.replacen(&exact, "", 1);
            }
        }
        raw
    }};
}
