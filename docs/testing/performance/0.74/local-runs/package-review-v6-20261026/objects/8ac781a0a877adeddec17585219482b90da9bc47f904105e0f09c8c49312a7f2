//! Exact test hooks and one later literal-prefix substitution, no old exclusions.
#[path = "fresh_status_guard.rs"]
pub(crate) mod fresh_status;
pub(crate) const HOOKS: [(&str,&str);2]=[
    ("tools/long-run-supervisor-074/src/diagnostic_kernel_admission_tests.rs","\n#[path = \"../tests/support/projection_prefix.rs\"]\npub(super) mod projection_prefix;\n"),
    ("tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs","\n#[path = \"../tests/support/projection_prefix_alloc.rs\"]\nmod projection_prefix;\n"),
];
pub(crate) const SOURCE: &str = "tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs";
pub(crate) const SUBSTITUTIONS: [(&str,&str);6]=[
    ("    let field = |prefix: &str| -> Result<Vec<u32>, KernelError> {\n        let mut lines = text.lines().filter_map(|line| line.strip_prefix(prefix));\n",
     "    let field = |name: &str| -> Result<Vec<u32>, KernelError> {\n        let prefix = format!(\"{name}:\");\n        let mut lines = text.lines().filter_map(|line| line.strip_prefix(&prefix));\n"),
    ("field(\"Pid:\")","field(\"Pid\")"),
    ("field(\"Tgid:\")","field(\"Tgid\")"),
    ("field(\"Uid:\")","field(\"Uid\")"),
    ("field(\"Gid:\")","field(\"Gid\")"),
    ("field(\"Groups:\")","field(\"Groups\")"),
];
pub(crate) fn normalize(path: &str, raw: &str) -> String {
    let mut raw = fresh_status::normalize(path, raw);
    if path == SOURCE {
        for (new, old) in SUBSTITUTIONS {
            assert_eq!(
                raw.matches(new).count(),
                1,
                "exact fixed prefix substitution: {new}"
            );
            raw = raw.replacen(new, old, 1);
        }
    }
    for (file, hook) in HOOKS {
        if path == file {
            assert_eq!(
                raw.matches(hook).count(),
                1,
                "exact projection prefix hook: {path}"
            );
            raw = raw.replacen(hook, "", 1);
        }
    }
    raw
}
