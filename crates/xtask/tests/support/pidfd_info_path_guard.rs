//! Exact pathname and hooks only; all old predicate bodies are reconstructed.
#[macro_export]
macro_rules! hc074_pidfd_info_path_view {
    ($path:expr,$source:expr) => {{
        let path: &str = $path;
        let mut raw: String = $source.to_owned();
        if path == "tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs" {
            let hook = "\n#[path = \"diagnostic_pidfd_info_path.rs\"]\nmod pidfd_info_path;\n";
            let new = ".open(pidfd_info_path::InfoPath::new(pidfd.as_raw_fd()).as_path())?;";
            if raw.contains("diagnostic_pidfd_info_path.rs") || raw.contains("pidfd_info_path::") {
                assert_eq!(raw.matches(hook).count(), 1);
                assert_eq!(raw.matches(new).count(), 1);
                raw = raw.replacen(hook, "", 1).replacen(
                    new,
                    ".open(format!(\"/proc/thread-self/fdinfo/{}\", pidfd.as_raw_fd()))?;",
                    1,
                );
            }
        }
        if path == "tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs"
            && raw.contains("mod pidfd_info_path;")
        {
            let hook =
                "\n#[path = \"../tests/support/pidfd_info_path_alloc.rs\"]\nmod pidfd_info_path;\n";
            assert_eq!(raw.matches(hook).count(), 1);
            raw = raw.replacen(hook, "", 1);
        }
        raw
    }};
}
