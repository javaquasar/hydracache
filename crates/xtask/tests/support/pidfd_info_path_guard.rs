//! Exact test hook only at baseline; no predicate/body exclusion.
#[macro_export]
macro_rules! hc074_pidfd_info_path_view {
    ($path:expr,$source:expr) => {{
        let path: &str = $path;
        let mut raw: String = $source.to_owned();
        if path == "tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs"
            && raw.contains("mod pidfd_info_path;")
        {
            let hook = "\n#[path = \"../tests/support/pidfd_info_path_alloc.rs\"]\nmod pidfd_info_path;\n";
            assert_eq!(raw.matches(hook).count(), 1);
            raw = raw.replacen(hook, "", 1);
        }
        raw
    }};
}
