//! Reversible literal status calls and test hooks, never a whole-body waiver.
#[path = "pid_path_guard.rs"]
pub(crate) mod pid_path;
pub(crate) const HOOKS: [(&str, &str); 2] = [
    ("tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs", "\n#[cfg(test)]\n#[path = \"../tests/support/fresh_kernel_cost.rs\"]\nmod fresh_kernel_cost;\n"),
    ("tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs", "\n#[path = \"../tests/support/fresh_status_alloc.rs\"]\nmod fresh_status;\n"),
];
pub(crate) const CHANGES: [(&str, &str, &str); 3] = [
    ("tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs", "pub(super) fn fixed_child(\n    parent: &File,\n    name: &std::ffi::CStr,\n    directory: bool,\n) -> Result<File, KernelError> {", "fn fixed_child(parent: &File, name: &std::ffi::CStr, directory: bool) -> Result<File, KernelError> {"),
    ("tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs", "let status = origin::fixed_child(&directory, c\"status\", false)?;", "let status = origin::child(&directory, \"status\", false)?;"),
    ("tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs", "let named = origin::fixed_child(&self.directory, c\"status\", false)?;", "let named = origin::child(&self.directory, \"status\", false)?;"),
];
pub(crate) fn normalize(path: &str, raw: &str) -> String {
    let mut raw = pid_path::normalize_legacy(path, raw);
    for (file, exact) in HOOKS {
        if path == file {
            assert_eq!(
                raw.matches(exact).count(),
                1,
                "exact fresh status test hook"
            );
            raw = raw.replacen(exact, "", 1);
        }
    }
    for (file, new, old) in CHANGES {
        if path == file {
            assert_eq!(raw.matches(new).count(), 1, "exact fixed status change");
            raw = raw.replacen(new, old, 1);
        }
    }
    raw
}
