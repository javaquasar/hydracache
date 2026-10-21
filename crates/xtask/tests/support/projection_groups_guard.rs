//! Reverse only exact inline group ownership edits; no predicate exclusions.
#[macro_export]
macro_rules! hc074_projection_groups_view {
    ($path:expr,$source:expr) => {{
        let path: &str = $path;
        let mut raw: String = $source.to_owned();
        if path == "tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs"
            && (raw.contains("diagnostic_projection_groups.rs") || raw.contains("projection_groups::"))
        {
            let hook = "#[path = \"diagnostic_projection_groups.rs\"]\nmod projection_groups;\n\n";
            let new = ".collect::<Result<projection_groups::Values, _>>()?\n            .into_vec();";
            assert_eq!(raw.matches(hook).count(), 1);
            assert_eq!(raw.matches(new).count(), 1);
            raw = raw.replacen(hook, "", 1).replacen(new, ".collect::<Result<Vec<_>, _>>()?;", 1);
        }
        if path == "tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs"
            && raw.contains("mod projection_groups;")
        {
            let hook = "\n#[path = \"../tests/support/projection_groups_alloc.rs\"]\nmod projection_groups;\n";
            assert_eq!(raw.matches(hook).count(), 1);
            raw = raw.replacen(hook, "", 1);
        }
        raw
    }};
}
