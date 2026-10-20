//! Exact test hooks and one later literal-prefix substitution, no old exclusions.
pub(crate) const HOOKS: [(&str,&str);2]=[
    ("tools/long-run-supervisor-074/src/diagnostic_kernel_admission_tests.rs","\n#[path = \"../tests/support/projection_prefix.rs\"]\npub(super) mod projection_prefix;\n"),
    ("tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs","\n#[path = \"../tests/support/projection_prefix_alloc.rs\"]\nmod projection_prefix;\n"),
];
pub(crate) fn normalize(path: &str, raw: &str) -> String {
    let mut raw = raw.to_owned();
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
